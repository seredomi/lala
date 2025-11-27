use crate::db::{
    create_asset, create_file, delete_file_and_assets, get_all_files, get_assets_by_file, DbPool,
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

    let asset_id = Uuid::new_v4().to_string();
    create_asset(
        &pool,
        &asset_id,
        &file_id,
        None,
        AssetType::Original,
        dest_path.to_str().unwrap(),
        ProcessingStatus::Queued,
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
