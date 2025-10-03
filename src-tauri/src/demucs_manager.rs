use crate::demucs_model::DemucsModel;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use tch::Tensor;

const DEMUCS_MODEL_PATH: &str = "models/hdemucs_high_musdb_plus.pt";

pub struct DemucsManager;

impl DemucsManager {
    pub async fn separate_audio(audio: Tensor) -> Result<HashMap<String, Tensor>> {
        let model = DemucsModel::new(Path::new(DEMUCS_MODEL_PATH))?;
        model.separate(&audio)
    }
}
