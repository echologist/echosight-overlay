//! Narrow, injectable native file-prompt boundary. No path is a write grant until
//! the save prompt resolves Some(path); import never writes to the selected file.
use gpui::{App, PathPromptOptions};
use std::{
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};

pub(super) type PathPrompt = Pin<Box<dyn Future<Output = Result<Option<PathBuf>, String>>>>;
pub(super) trait TemplatePorts {
    fn prompt_open_file(&self, cx: &mut App) -> PathPrompt;
    fn prompt_save_file(&self, directory: &Path, filename: &str, cx: &mut App) -> PathPrompt;
}
pub(super) struct NativeTemplatePorts;
impl TemplatePorts for NativeTemplatePorts {
    fn prompt_open_file(&self, cx: &mut App) -> PathPrompt {
        let response = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Import template JSON".into()),
        });
        Box::pin(async move {
            response
                .await
                .map_err(|e| e.to_string())?
                .map(|paths| paths.and_then(|paths| paths.into_iter().next()))
                .map_err(|e| e.to_string())
        })
    }
    fn prompt_save_file(&self, directory: &Path, filename: &str, cx: &mut App) -> PathPrompt {
        let response = cx.prompt_for_new_path(directory, Some(filename));
        Box::pin(async move {
            response
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())
        })
    }
}
