mod audio_io;
mod commands;
mod config;
mod db;
mod demucs_model;
mod models;
mod processing;
mod worker;

use commands::{
    cancel_processing, delete_file, download_asset, list_assets, list_files, process_to_stage,
    upload_file,
};
use config::get_app_config;
use db::{init_db, reset_interrupted_jobs};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            setup_app(app_handle)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_app_config,
            upload_file,
            list_files,
            list_assets,
            download_asset,
            delete_file,
            process_to_stage,
            cancel_processing,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn setup_app(app: AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    // get app data directory
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("failed to get app data dir");

    std::fs::create_dir_all(&app_data_dir)?;

    let db_path = app_data_dir.join("lala.db");
    let pool = init_db(&db_path)?;

    // startup recovery: reset any jobs that were processing when app last closed
    let reset_count = reset_interrupted_jobs(&pool)?;
    if reset_count > 0 {
        println!("reset {} interrupted jobs to queued", reset_count);
    }

    // manage state
    app.manage(pool.clone());
    app.manage(app_data_dir.clone());

    // start background worker
    let shutdown = Arc::new(AtomicBool::new(false));
    worker::start_worker(app.clone(), pool.clone(), shutdown.clone());

    Ok(())
}
