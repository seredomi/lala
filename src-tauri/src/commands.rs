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

    // determine what stages exist
    let has_original = assets
        .iter()
        .any(|a| matches!(a.asset_type, AssetType::Original));
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
            // need stems first
            if !has_stems {
                let original = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Original))
                    .ok_or("original asset not found")?;

                crate::db::update_asset_status(&pool, &original.id, ProcessingStatus::Queued, None)
                    .map_err(|e| e.to_string())?;
            }

            // queue midi creation
            if !has_midi {
                let midi_id = Uuid::new_v4().to_string();
                let midi_path = file_dir.join("stem_piano.midi");

                // find piano stem to use as parent
                let piano_stem = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::StemPiano));

                create_asset(
                    &pool,
                    &midi_id,
                    &file_id,
                    piano_stem.map(|s| s.id.as_str()),
                    AssetType::Midi,
                    midi_path.to_str().unwrap(),
                    ProcessingStatus::Queued,
                )
                .map_err(|e| e.to_string())?;
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
            }

            // queue midi creation
            if !has_midi {
                let midi_id = Uuid::new_v4().to_string();
                let midi_path = file_dir.join("stem_piano.midi");

                let piano_stem = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::StemPiano));

                create_asset(
                    &pool,
                    &midi_id,
                    &file_id,
                    piano_stem.map(|s| s.id.as_str()),
                    AssetType::Midi,
                    midi_path.to_str().unwrap(),
                    ProcessingStatus::Queued,
                )
                .map_err(|e| e.to_string())?;
            }

            // queue pdf creation
            let has_pdf = assets.iter().any(|a| {
                matches!(a.asset_type, AssetType::Pdf)
                    && matches!(a.status, ProcessingStatus::Completed)
            });

            if !has_pdf {
                let pdf_id = Uuid::new_v4().to_string();
                let pdf_path = file_dir.join("stem_piano.pdf");

                let midi_asset = assets
                    .iter()
                    .find(|a| matches!(a.asset_type, AssetType::Midi));

                create_asset(
                    &pool,
                    &pdf_id,
                    &file_id,
                    midi_asset.map(|m| m.id.as_str()),
                    AssetType::Pdf,
                    pdf_path.to_str().unwrap(),
                    ProcessingStatus::Queued,
                )
                .map_err(|e| e.to_string())?;
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
