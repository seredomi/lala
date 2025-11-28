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
    let file_dir = app_data_dir.join("processing-files").join(&file_id);

    // determine what stages exist and are completed
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

    // check if anything is already queued or processing
    let has_queued_or_processing = assets.iter().any(|a| {
        matches!(
            a.status,
            ProcessingStatus::Queued | ProcessingStatus::Processing
        )
    });

    if has_queued_or_processing {
        return Err("file already has processing in progress".to_string());
    }

    if !has_original {
        return Err("no original file found".to_string());
    }

    match target_stage.as_str() {
        "stems" => {
            if !has_stems {
                // queue separation on the original asset
                let original = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Original))
                    .ok_or("original asset not found")?;

                crate::db::update_asset_status(&pool, &original.id, ProcessingStatus::Queued, None)
                    .map_err(|e| e.to_string())?;
            }
        }
        "midi" => {
            // need stems first - only queue original, don't create midi yet
            if !has_stems {
                let original = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Original))
                    .ok_or("original asset not found")?;

                crate::db::update_asset_status(&pool, &original.id, ProcessingStatus::Queued, None)
                    .map_err(|e| e.to_string())?;

                // don't create midi asset yet - it will be created after stems complete
                return Ok(());
            }

            // stems are ready, now queue midi if it doesn't exist
            let existing_midi = assets
                .iter()
                .find(|a| matches!(a.asset_type, AssetType::Midi));

            if existing_midi.is_none() {
                let midi_id = Uuid::new_v4().to_string();
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
            } else if let Some(midi) = existing_midi {
                if matches!(
                    midi.status,
                    ProcessingStatus::Failed | ProcessingStatus::Cancelled
                ) {
                    crate::db::update_asset_status(&pool, &midi.id, ProcessingStatus::Queued, None)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        "pdf" => {
            // need stems first
            if !has_stems {
                let original = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Original))
                    .ok_or("original asset not found")?;

                crate::db::update_asset_status(&pool, &original.id, ProcessingStatus::Queued, None)
                    .map_err(|e| e.to_string())?;

                return Ok(());
            }

            // stems ready, need midi
            let existing_midi = assets
                .iter()
                .find(|a| matches!(a.asset_type, AssetType::Midi));

            if existing_midi.is_none()
                || matches!(
                    existing_midi.unwrap().status,
                    ProcessingStatus::Failed | ProcessingStatus::Cancelled
                )
            {
                let midi_id = if let Some(midi) = existing_midi {
                    // requeue existing
                    crate::db::update_asset_status(&pool, &midi.id, ProcessingStatus::Queued, None)
                        .map_err(|e| e.to_string())?;
                    midi.id.clone()
                } else {
                    // create new
                    let midi_id = Uuid::new_v4().to_string();
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

                    midi_id
                };

                // don't create pdf yet - wait for midi to complete
                return Ok(());
            }

            // midi is completed, now queue pdf
            if !has_midi {
                return Ok(()); // midi is processing, wait
            }

            let existing_pdf = assets
                .iter()
                .find(|a| matches!(a.asset_type, AssetType::Pdf));

            if existing_pdf.is_none() {
                let pdf_id = Uuid::new_v4().to_string();
                let pdf_path = file_dir.join("stem_piano.pdf");

                let midi_asset = assets
                    .iter()
                    .find(|a| {
                        matches!(a.asset_type, AssetType::Midi)
                            && matches!(a.status, ProcessingStatus::Completed)
                    })
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
            } else if let Some(pdf) = existing_pdf {
                if matches!(
                    pdf.status,
                    ProcessingStatus::Failed | ProcessingStatus::Cancelled
                ) {
                    crate::db::update_asset_status(&pool, &pdf.id, ProcessingStatus::Queued, None)
                        .map_err(|e| e.to_string())?;
                }
            }
        }
        _ => return Err("invalid target stage".to_string()),
    }

    Ok(())
}

#[command]
pub async fn cancel_processing(
    pool: tauri::State<'_, DbPool>,
    file_id: String,
) -> Result<(), String> {
    cancel_file_processing(&pool, &file_id).map_err(|e| e.to_string())
}
