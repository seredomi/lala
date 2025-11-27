use crate::db::{get_next_queued_asset, update_asset_status, DbPool};
use crate::models::{AssetType, ProcessingStatus};
use crate::processing::{midi_to_pdf, separate_audio, transcribe_to_midi};
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(serde::Serialize, Clone)]
pub struct ProcessingProgress {
    pub file_id: String,
    pub asset_id: String,
    pub asset_type: String,
    pub title: String,
    pub description: String,
}

pub fn start_worker(app: AppHandle, pool: DbPool, shutdown: Arc<AtomicBool>) {
    thread::spawn(move || {
        println!("background worker started");

        while !shutdown.load(Ordering::Relaxed) {
            match process_next_job(&app, &pool) {
                Ok(had_job) => {
                    if !had_job {
                        // no jobs, sleep briefly
                        thread::sleep(Duration::from_millis(500));
                    }
                }
                Err(e) => {
                    eprintln!("worker error: {:?}", e);
                    thread::sleep(Duration::from_secs(1));
                }
            }
        }

        println!("background worker stopped");
    });
}

fn process_next_job(app: &AppHandle, pool: &DbPool) -> Result<bool> {
    let asset = get_next_queued_asset(pool)?;

    if let Some(asset) = asset {
        println!(
            "processing asset: {} (type: {:?})",
            asset.id, asset.asset_type
        );

        // mark as processing
        update_asset_status(pool, &asset.id, ProcessingStatus::Processing, None)?;

        emit_progress(
            app,
            &asset.file_id,
            &asset.id,
            &asset.asset_type,
            "processing...",
            &format!("working on {:?}", asset.asset_type),
        );

        // dispatch based on type
        let result = match asset.asset_type {
            AssetType::Original => process_separation(app, pool, &asset),
            AssetType::StemPiano => process_transcription(app, pool, &asset),
            AssetType::Midi => process_pdf_conversion(app, pool, &asset),
            _ => {
                // other stems don't have follow-up processing
                update_asset_status(pool, &asset.id, ProcessingStatus::Completed, None)?;
                Ok(())
            }
        };

        match result {
            Ok(_) => {
                update_asset_status(pool, &asset.id, ProcessingStatus::Completed, None)?;
                emit_progress(
                    app,
                    &asset.file_id,
                    &asset.id,
                    &asset.asset_type,
                    "completed",
                    &format!("{:?} ready", asset.asset_type),
                );
            }
            Err(e) => {
                let err_msg = format!("{:?}", e);
                eprintln!("job failed: {}", err_msg);
                update_asset_status(pool, &asset.id, ProcessingStatus::Failed, Some(&err_msg))?;
                emit_progress(
                    app,
                    &asset.file_id,
                    &asset.id,
                    &asset.asset_type,
                    "failed",
                    &err_msg,
                );
            }
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

fn process_separation(_app: &AppHandle, pool: &DbPool, asset: &crate::models::Asset) -> Result<()> {
    use crate::db::create_asset;
    use uuid::Uuid;

    let input_path = Path::new(&asset.file_path);
    let output_dir = input_path.parent().unwrap();
    let model_path = Path::new("models/hdemucs.pt");

    let stem_paths = separate_audio(input_path, output_dir, model_path)?;

    // create asset records for each stem
    for (stem_name, stem_path) in stem_paths {
        let asset_type = match stem_name.as_str() {
            "piano" => AssetType::StemPiano,
            "vocals" => AssetType::StemVocals,
            "drums" => AssetType::StemDrums,
            "bass" => AssetType::StemBass,
            _ => continue,
        };

        let stem_id = Uuid::new_v4().to_string();

        create_asset(
            pool,
            &stem_id,
            &asset.file_id,
            Some(&asset.id),
            asset_type.clone(),
            &stem_path,
            ProcessingStatus::Completed,
        )?;

        // if it's piano, queue transcription
        if matches!(asset_type, AssetType::StemPiano) {
            let midi_id = Uuid::new_v4().to_string();
            let midi_path = output_dir.join("stem_piano.midi");

            create_asset(
                pool,
                &midi_id,
                &asset.file_id,
                Some(&stem_id),
                AssetType::Midi,
                midi_path.to_str().unwrap(),
                ProcessingStatus::Queued,
            )?;
        }
    }

    Ok(())
}

fn process_transcription(
    _app: &AppHandle,
    pool: &DbPool,
    asset: &crate::models::Asset,
) -> Result<()> {
    use crate::db::create_asset;
    use uuid::Uuid;

    let input_wav = Path::new(&asset.file_path);
    let output_dir = input_wav.parent().unwrap();

    // find the midi asset that's queued
    let assets = crate::db::get_assets_by_file(pool, &asset.file_id)?;
    let midi_asset = assets
        .iter()
        .find(|a| {
            matches!(a.asset_type, AssetType::Midi) && matches!(a.status, ProcessingStatus::Queued)
        })
        .ok_or_else(|| anyhow::anyhow!("no queued midi asset found"))?;

    let midi_path = Path::new(&midi_asset.file_path);
    transcribe_to_midi(input_wav, midi_path)?;

    // update midi asset to completed
    update_asset_status(pool, &midi_asset.id, ProcessingStatus::Completed, None)?;

    // queue pdf conversion
    let pdf_id = Uuid::new_v4().to_string();
    let pdf_path = output_dir.join("stem_piano.pdf");

    create_asset(
        pool,
        &pdf_id,
        &asset.file_id,
        Some(&midi_asset.id),
        AssetType::Pdf,
        pdf_path.to_str().unwrap(),
        ProcessingStatus::Queued,
    )?;

    Ok(())
}

fn process_pdf_conversion(
    _app: &AppHandle,
    pool: &DbPool,
    asset: &crate::models::Asset,
) -> Result<()> {
    // find parent midi asset
    let parent_id = asset
        .parent_asset_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("pdf asset has no parent"))?;

    let assets = crate::db::get_assets_by_file(pool, &asset.file_id)?;
    let midi_asset = assets
        .iter()
        .find(|a| &a.id == parent_id)
        .ok_or_else(|| anyhow::anyhow!("parent midi asset not found"))?;

    let midi_path = Path::new(&midi_asset.file_path);
    let pdf_path = Path::new(&asset.file_path);

    midi_to_pdf(midi_path, pdf_path)?;

    Ok(())
}

fn emit_progress(
    app: &AppHandle,
    file_id: &str,
    asset_id: &str,
    asset_type: &AssetType,
    title: &str,
    description: &str,
) {
    let _ = app.emit(
        "processing_progress",
        ProcessingProgress {
            file_id: file_id.to_string(),
            asset_id: asset_id.to_string(),
            asset_type: asset_type.to_string(),
            title: title.to_string(),
            description: description.to_string(),
        },
    );
}
