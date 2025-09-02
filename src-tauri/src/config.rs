use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct FileUploadConfig {
    pub max_file_size_mb: u32,
    pub permitted_file_extensions: [&'static str; 1],
    pub max_upload_time_sec: u16,
}

#[derive(Serialize, Clone)]
pub struct AppConfig {
    pub file_upload: FileUploadConfig,
}

#[tauri::command]
pub fn get_app_config() -> AppConfig {
    AppConfig {
        file_upload: FileUploadConfig {
            max_file_size_mb: 500,
            permitted_file_extensions: [".wav"],
            max_upload_time_sec: 300,
        },
    }
}
