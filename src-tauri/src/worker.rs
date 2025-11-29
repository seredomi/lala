use crate::db::{
    create_asset, get_assets_by_file, get_next_queued_asset, update_asset_status, DbPool,
};
use crate::models::{AssetType, ProcessingStatus};
use crate::processing::{midi_to_pdf, separate_audio, transcribe_to_midi};
use anyhow::Result;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

#[derive(serde::Serialize, Clone)]
pub struct ProcessingProgress {
    pub file_id: String,
    pub asset_id: String,
    pub asset_type: String,
    pub title: String,
    pub description: String,
    pub progress: f32, // 0.0 to 1.0
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
            "processing",
            &format!("working on {:?}", asset.asset_type),
            0.0,
        );

        // dispatch based on type
        let result = match asset.asset_type {
            AssetType::Original => process_separation(app, pool, &asset),
            AssetType::Midi => process_transcription(app, pool, &asset),
            AssetType::Pdf => process_pdf_conversion(app, pool, &asset),
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
                    1.0,
                );

                // check if we should queue the next stage
                let _ = queue_next_stage_for_target(pool, &asset);
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
                    0.0,
                );

                // clear target stage on failure
                let _ = crate::db::set_target_stage(pool, &asset.file_id, None);
            }
        }

        Ok(true)
    } else {
        Ok(false)
    }
}

fn queue_next_stage_for_target(
    pool: &DbPool,
    completed_asset: &crate::models::Asset,
) -> Result<()> {
    use crate::db::{get_file_target_stage, set_target_stage};
    use std::path::PathBuf;

    // get the target stage for this file
    let target_stage = match get_file_target_stage(pool, &completed_asset.file_id)? {
        Some(target) => target,
        None => return Ok(()), // no target set, nothing to do
    };

    println!("checking if next stage needed for target: {}", target_stage);

    // get all assets for this file
    let assets = get_assets_by_file(pool, &completed_asset.file_id)?;

    let has_stems = assets.iter().any(|a| {
        matches!(a.asset_type, AssetType::StemPiano)
            && matches!(a.status, ProcessingStatus::Completed)
    });

    let has_midi = assets.iter().any(|a| {
        matches!(a.asset_type, AssetType::Midi) && matches!(a.status, ProcessingStatus::Completed)
    });

    let has_pdf = assets.iter().any(|a| {
        matches!(a.asset_type, AssetType::Pdf) && matches!(a.status, ProcessingStatus::Completed)
    });

    // determine if target is reached
    let target_reached = match target_stage.as_str() {
        "stems" => has_stems,
        "midi" => has_midi,
        "pdf" => has_pdf,
        _ => false,
    };

    if target_reached {
        println!("target stage '{}' reached, clearing target", target_stage);
        set_target_stage(pool, &completed_asset.file_id, None)?;
        return Ok(());
    }

    // figure out what to queue next
    match target_stage.as_str() {
        "stems" => {
            // shouldn't get here, but just in case
            println!("target is stems but not reached yet");
        }
        "midi" => {
            if has_stems && !has_midi {
                // queue midi
                let existing_midi = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Midi));

                if existing_midi.is_none() {
                    println!("creating and queueing midi asset");
                    let midi_id = Uuid::new_v4().to_string();

                    // construct path
                    let file_path = &completed_asset.file_path;
                    let file_dir = PathBuf::from(file_path).parent().unwrap().to_path_buf();
                    let midi_path = file_dir.join("stem_piano.midi");

                    let piano_stem = assets
                        .iter()
                        .find(|a| matches!(a.asset_type, AssetType::StemPiano))
                        .ok_or_else(|| anyhow::anyhow!("piano stem not found"))?;

                    create_asset(
                        pool,
                        &midi_id,
                        &completed_asset.file_id,
                        Some(&piano_stem.id),
                        AssetType::Midi,
                        midi_path.to_str().unwrap(),
                        ProcessingStatus::Queued,
                    )?;
                }
            }
        }
        "pdf" => {
            if has_stems && !has_midi {
                // queue midi first
                let existing_midi = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Midi));

                if existing_midi.is_none() {
                    println!("creating and queueing midi asset (for pdf)");
                    let midi_id = Uuid::new_v4().to_string();

                    let file_path = &completed_asset.file_path;
                    let file_dir = PathBuf::from(file_path).parent().unwrap().to_path_buf();
                    let midi_path = file_dir.join("stem_piano.midi");

                    let piano_stem = assets
                        .iter()
                        .find(|a| matches!(a.asset_type, AssetType::StemPiano))
                        .ok_or_else(|| anyhow::anyhow!("piano stem not found"))?;

                    create_asset(
                        pool,
                        &midi_id,
                        &completed_asset.file_id,
                        Some(&piano_stem.id),
                        AssetType::Midi,
                        midi_path.to_str().unwrap(),
                        ProcessingStatus::Queued,
                    )?;
                }
            } else if has_midi && !has_pdf {
                // queue pdf
                let existing_pdf = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Pdf));

                if existing_pdf.is_none() {
                    println!("creating and queueing pdf asset");
                    let pdf_id = Uuid::new_v4().to_string();

                    let file_path = &completed_asset.file_path;
                    let file_dir = PathBuf::from(file_path).parent().unwrap().to_path_buf();
                    let pdf_path = file_dir.join("stem_piano.pdf");

                    let midi_asset = assets
                        .iter()
                        .find(|a| matches!(a.asset_type, AssetType::Midi))
                        .ok_or_else(|| anyhow::anyhow!("midi asset not found"))?;

                    create_asset(
                        pool,
                        &pdf_id,
                        &completed_asset.file_id,
                        Some(&midi_asset.id),
                        AssetType::Pdf,
                        pdf_path.to_str().unwrap(),
                        ProcessingStatus::Queued,
                    )?;
                }
            }
        }
        _ => {}
    }

    Ok(())
}

fn process_separation(app: &AppHandle, pool: &DbPool, asset: &crate::models::Asset) -> Result<()> {
    let input_path = Path::new(&asset.file_path);
    let output_dir = input_path.parent().unwrap();
    let model_path = Path::new("models/hdemucs.pt");

    let app_clone = app.clone();
    let file_id = asset.file_id.clone();
    let asset_id = asset.id.clone();

    let stem_paths = separate_audio(input_path, output_dir, model_path, |progress| {
        emit_progress(
            &app_clone,
            &file_id,
            &asset_id,
            &AssetType::Original,
            "separating",
            &format!("processing audio: {:.0}%", progress * 100.0),
            progress,
        );
    })?;

    // create asset records for each stem (all marked as completed)
    for (stem_name, stem_path) in stem_paths {
        let asset_type = match stem_name.as_str() {
            "other" => AssetType::StemPiano,
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
            asset_type,
            &stem_path,
            ProcessingStatus::Completed,
        )?;
    }

    Ok(())
}

fn process_transcription(
    app: &AppHandle,
    pool: &DbPool,
    asset: &crate::models::Asset,
) -> Result<()> {
    // find parent piano stem
    let parent_id = asset
        .parent_asset_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("midi asset has no parent"))?;

    let assets = get_assets_by_file(pool, &asset.file_id)?;
    let piano_stem = assets
        .iter()
        .find(|a| &a.id == parent_id)
        .ok_or_else(|| anyhow::anyhow!("parent piano stem not found"))?;

    let input_wav = Path::new(&piano_stem.file_path);
    let midi_path = Path::new(&asset.file_path);

    let app_clone = app.clone();
    let file_id = asset.file_id.clone();
    let asset_id = asset.id.clone();

    transcribe_to_midi(input_wav, midi_path, |progress| {
        emit_progress(
            &app_clone,
            &file_id,
            &asset_id,
            &AssetType::Midi,
            "transcribing",
            &format!("converting to MIDI: {:.0}%", progress * 100.0),
            progress,
        );
    })?;

    Ok(())
}

fn process_pdf_conversion(
    app: &AppHandle,
    pool: &DbPool,
    asset: &crate::models::Asset,
) -> Result<()> {
    // find parent midi asset
    let parent_id = asset
        .parent_asset_id
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("pdf asset has no parent"))?;

    let assets = get_assets_by_file(pool, &asset.file_id)?;
    let midi_asset = assets
        .iter()
        .find(|a| &a.id == parent_id)
        .ok_or_else(|| anyhow::anyhow!("parent midi asset not found"))?;

    let midi_path = Path::new(&midi_asset.file_path);
    let pdf_path = Path::new(&asset.file_path);

    let app_clone = app.clone();
    let file_id = asset.file_id.clone();
    let asset_id = asset.id.clone();

    midi_to_pdf(midi_path, pdf_path, |progress| {
        emit_progress(
            &app_clone,
            &file_id,
            &asset_id,
            &AssetType::Pdf,
            "converting",
            &format!("generating sheet music: {:.0}%", progress * 100.0),
            progress,
        );
    })?;

    Ok(())
}

fn emit_progress(
    app: &AppHandle,
    file_id: &str,
    asset_id: &str,
    asset_type: &AssetType,
    title: &str,
    description: &str,
    progress: f32,
) {
    let _ = app.emit(
        "processing_progress",
        ProcessingProgress {
            file_id: file_id.to_string(),
            asset_id: asset_id.to_string(),
            asset_type: asset_type.to_string(),
            title: title.to_string(),
            description: description.to_string(),
            progress,
        },
    );
}
