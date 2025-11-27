use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessingStatus {
    Queued,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl ProcessingStatus {
    pub fn to_string(&self) -> String {
        match self {
            ProcessingStatus::Queued => "queued".to_string(),
            ProcessingStatus::Processing => "processing".to_string(),
            ProcessingStatus::Completed => "completed".to_string(),
            ProcessingStatus::Failed => "failed".to_string(),
            ProcessingStatus::Cancelled => "cancelled".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "queued" => ProcessingStatus::Queued,
            "processing" => ProcessingStatus::Processing,
            "completed" => ProcessingStatus::Completed,
            "failed" => ProcessingStatus::Failed,
            "cancelled" => ProcessingStatus::Cancelled,
            _ => ProcessingStatus::Failed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetType {
    Original,
    StemPiano,
    StemVocals,
    StemDrums,
    StemBass,
    Midi,
    Pdf,
}

impl AssetType {
    pub fn to_string(&self) -> String {
        match self {
            AssetType::Original => "original".to_string(),
            AssetType::StemPiano => "stem_piano".to_string(),
            AssetType::StemVocals => "stem_vocals".to_string(),
            AssetType::StemDrums => "stem_drums".to_string(),
            AssetType::StemBass => "stem_bass".to_string(),
            AssetType::Midi => "midi".to_string(),
            AssetType::Pdf => "pdf".to_string(),
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "original" => AssetType::Original,
            "stem_piano" => AssetType::StemPiano,
            "stem_vocals" => AssetType::StemVocals,
            "stem_drums" => AssetType::StemDrums,
            "stem_bass" => AssetType::StemBass,
            "midi" => AssetType::Midi,
            "pdf" => AssetType::Pdf,
            _ => AssetType::Original,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FileRecord {
    pub id: String,
    pub original_filename: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Asset {
    pub id: String,
    pub file_id: String,
    pub parent_asset_id: Option<String>,
    pub asset_type: AssetType,
    pub file_path: String,
    pub status: ProcessingStatus,
    pub error_message: Option<String>,
    pub created_at: i64,
}
