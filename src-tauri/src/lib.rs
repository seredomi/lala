mod audio_io;
mod config;
mod dsp;
mod separation;
mod separator;

use config::get_app_config;
use separation::{abort_separation, start_separation};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_app_config,
            start_separation,
            abort_separation
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
