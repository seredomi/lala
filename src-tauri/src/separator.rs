use crate::demucs_model::DemucsModel;
use crate::separation::LoadingState;
use anyhow::Result;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tch::Tensor;

pub struct Separator {
    model: DemucsModel,
}

impl Separator {
    pub fn new() -> Result<Self> {
        let model_path = Path::new("models/demucs.pt");
        let model = DemucsModel::new(model_path)?;

        Ok(Self { model })
    }

    pub fn separate(
        &self,
        audio: Tensor,
        app_handle: &AppHandle,
        abort_flag: Arc<AtomicBool>,
    ) -> Result<HashMap<String, Tensor>> {
        if abort_flag.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("separation aborted by user"));
        }

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "processing with ai model...".to_string(),
                description: "running demucs separation".to_string(),
                progress: Some(30),
            },
        )?;

        let separated = self.model.separate(&audio)?;

        if abort_flag.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("separation aborted by user"));
        }

        app_handle.emit(
            "separation_progress",
            &LoadingState {
                title: "creating instrumental track...".to_string(),
                description: "combining non-vocal stems".to_string(),
                progress: Some(85),
            },
        )?;

        // create instrumental by combining all non-vocal stems
        let mut result = separated;
        if let (Some(drums), Some(bass), Some(other)) =
            (result.get("drums"), result.get("bass"), result.get("other"))
        {
            let instrumental = drums + bass + other;
            result.insert("instrumental".to_string(), instrumental);
        }

        Ok(result)
    }
}
