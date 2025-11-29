use crate::db::{
    cancel_file_processing, create_asset, create_file, delete_file_and_assets, get_all_files,
    get_assets_by_file, DbPool,
};
use crate::models::{Asset, AssetType, FileRecord, ProcessingStatus};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use tauri::command;
use uuid::Uuid;

#[command]
pub async fn upload_file(
    pool: tauri::State<'_, DbPool>,
    app_data_dir: tauri::State<'_, PathBuf>,
    source_path: String,
    original_filename: String,
) -> Result<String, String> {
    let file_id = Uuid::new_v4().to_string();
    let file_dir = app_data_dir.join("processing-files").join(&file_id);

    fs::create_dir_all(&file_dir).map_err(|e| format!("failed to create directory: {:?}", e))?;

    let dest_path = file_dir.join("original.wav");
    fs::copy(&source_path, &dest_path).map_err(|e| format!("failed to copy file: {:?}", e))?;

    create_file(&pool, &file_id, &original_filename).map_err(|e| e.to_string())?;

    // create original asset as completed (not queued - user must explicitly start processing)
    let asset_id = Uuid::new_v4().to_string();
    create_asset(
        &pool,
        &asset_id,
        &file_id,
        None,
        AssetType::Original,
        dest_path.to_str().unwrap(),
        ProcessingStatus::Completed,
    )
    .map_err(|e| e.to_string())?;

    Ok(file_id)
}

#[command]
pub async fn list_files(pool: tauri::State<'_, DbPool>) -> Result<Vec<FileRecord>, String> {
    get_all_files(&pool).map_err(|e| e.to_string())
}

#[command]
pub async fn list_assets(
    pool: tauri::State<'_, DbPool>,
    file_id: String,
) -> Result<Vec<Asset>, String> {
    get_assets_by_file(&pool, &file_id).map_err(|e| e.to_string())
}

#[command]
pub async fn download_asset(asset_path: String, destination: String) -> Result<(), String> {
    fs::copy(&asset_path, &destination).map_err(|e| format!("failed to copy file: {:?}", e))?;
    Ok(())
}

#[command]
pub async fn delete_file(
    pool: tauri::State<'_, DbPool>,
    app_data_dir: tauri::State<'_, PathBuf>,
    file_id: String,
) -> Result<(), String> {
    delete_file_and_assets(&pool, &file_id).map_err(|e| e.to_string())?;

    let file_dir = app_data_dir.join("processing-files").join(&file_id);
    if file_dir.exists() {
        fs::remove_dir_all(&file_dir)
            .map_err(|e| format!("failed to delete directory: {:?}", e))?;
    }

    Ok(())
}

#[command]
pub async fn process_to_stage(
    pool: tauri::State<'_, DbPool>,
    app_data_dir: tauri::State<'_, PathBuf>,
    file_id: String,
    target_stage: String,
) -> Result<(), String> {
    let assets = get_assets_by_file(&pool, &file_id).map_err(|e| e.to_string())?;

    // check if anything is already processing
    let has_processing = assets
        .iter()
        .any(|a| matches!(a.status, ProcessingStatus::Processing));

    if has_processing {
        return Err("file already has processing in progress".to_string());
    }

    // validate target stage
    if !matches!(target_stage.as_str(), "stems" | "midi" | "pdf") {
        return Err("invalid target stage".to_string());
    }

    // set the target stage on the file
    crate::db::set_target_stage(&pool, &file_id, Some(&target_stage)).map_err(|e| e.to_string())?;

    // determine what needs to happen first
    let has_original = assets.iter().any(|a| {
        matches!(a.asset_type, AssetType::Original)
            && matches!(a.status, ProcessingStatus::Completed)
    });

    let has_stems = assets.iter().any(|a| {
        matches!(a.asset_type, AssetType::StemPiano)
            && matches!(a.status, ProcessingStatus::Completed)
    });

    let has_midi = assets.iter().any(|a| {
        matches!(a.asset_type, AssetType::Midi) && matches!(a.status, ProcessingStatus::Completed)
    });

    if !has_original {
        return Err("no original file found".to_string());
    }

    // queue the first step that needs to happen
    if !has_stems {
        // need to separate stems first
        let original = assets
            .iter()
            .find(|a| matches!(a.asset_type, AssetType::Original))
            .ok_or("original asset not found")?;

        crate::db::update_asset_status(&pool, &original.id, ProcessingStatus::Queued, None)
            .map_err(|e| e.to_string())?;
    } else if target_stage != "stems" && !has_midi {
        // stems exist, but need midi
        let existing_midi = assets
            .iter()
            .find(|a| matches!(a.asset_type, AssetType::Midi));

        if let Some(midi) = existing_midi {
            // re-queue if failed/cancelled
            if matches!(
                midi.status,
                ProcessingStatus::Failed | ProcessingStatus::Cancelled
            ) {
                crate::db::update_asset_status(&pool, &midi.id, ProcessingStatus::Queued, None)
                    .map_err(|e| e.to_string())?;
            }
        } else {
            // create and queue midi asset
            let midi_id = Uuid::new_v4().to_string();
            let file_dir = app_data_dir.join("processing-files").join(&file_id);
            let midi_path = file_dir.join("stem_piano.midi");

            let piano_stem = assets
                .iter()
                .find(|a| matches!(a.asset_type, AssetType::StemPiano))
                .ok_or("piano stem not found")?;

            create_asset(
                &pool,
                &midi_id,
                &file_id,
                Some(&piano_stem.id),
                AssetType::Midi,
                midi_path.to_str().unwrap(),
                ProcessingStatus::Queued,
            )
            .map_err(|e| e.to_string())?;
        }
    } else if target_stage == "pdf" && has_midi {
        // midi exists, queue pdf
        let existing_pdf = assets
            .iter()
            .find(|a| matches!(a.asset_type, AssetType::Pdf));

        if let Some(pdf) = existing_pdf {
            if matches!(
                pdf.status,
                ProcessingStatus::Failed | ProcessingStatus::Cancelled
            ) {
                crate::db::update_asset_status(&pool, &pdf.id, ProcessingStatus::Queued, None)
                    .map_err(|e| e.to_string())?;
            }
        } else {
            let pdf_id = Uuid::new_v4().to_string();
            let file_dir = app_data_dir.join("processing-files").join(&file_id);
            let pdf_path = file_dir.join("stem_piano.pdf");

            let midi_asset = assets
                .iter()
                .find(|a| matches!(a.asset_type, AssetType::Midi))
                .ok_or("midi asset not found")?;

            create_asset(
                &pool,
                &pdf_id,
                &file_id,
                Some(&midi_asset.id),
                AssetType::Pdf,
                pdf_path.to_str().unwrap(),
                ProcessingStatus::Queued,
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

#[command]
pub async fn cancel_processing(
    pool: tauri::State<'_, DbPool>,
    file_id: String,
) -> Result<(), String> {
    // clear target stage
    crate::db::set_target_stage(&pool, &file_id, None).map_err(|e| e.to_string())?;

    // cancel any queued/processing assets
    cancel_file_processing(&pool, &file_id).map_err(|e| e.to_string())
}
