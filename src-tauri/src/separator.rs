use crate::demucs_manager::DemucsManager;
use crate::separation::LoadingState;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tch::Tensor;

pub struct Separator;

impl Separator {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    pub fn separate(
        &self,
        audio: Tensor,
        app: &AppHandle,
        abort_flag: Arc<AtomicBool>,
    ) -> Result<HashMap<String, Tensor>> {
        if abort_flag.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("separation aborted"));
        }

        app.emit(
            "separation_progress",
            &LoadingState {
                title: "processing audio...".to_string(),
                description: "running separation inference".to_string(),
                progress: Some(40),
            },
        )?;

        // Use tokio::task::spawn_blocking for CPU-intensive work
        let separated = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { DemucsManager::separate_audio(audio).await })
        })?;

        if abort_flag.load(Ordering::SeqCst) {
            return Err(anyhow::anyhow!("separation aborted"));
        }

        Ok(separated)
    }
}
