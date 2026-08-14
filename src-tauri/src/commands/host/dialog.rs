use tauri::Runtime;
use tauri_plugin_dialog::{FileDialogBuilder, FilePath, MessageDialogBuilder};

pub async fn pick_file<R: Runtime>(builder: FileDialogBuilder<R>) -> Option<FilePath> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    builder.pick_file(move |selection| {
        let _ = sender.send(selection);
    });
    receiver.await.unwrap_or(None)
}

pub async fn pick_files<R: Runtime>(builder: FileDialogBuilder<R>) -> Option<Vec<FilePath>> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    builder.pick_files(move |selection| {
        let _ = sender.send(selection);
    });
    receiver.await.unwrap_or(None)
}

pub async fn pick_folder<R: Runtime>(builder: FileDialogBuilder<R>) -> Option<FilePath> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    builder.pick_folder(move |selection| {
        let _ = sender.send(selection);
    });
    receiver.await.unwrap_or(None)
}

pub async fn save_file<R: Runtime>(builder: FileDialogBuilder<R>) -> Option<FilePath> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    builder.save_file(move |selection| {
        let _ = sender.send(selection);
    });
    receiver.await.unwrap_or(None)
}

pub async fn show_message<R: Runtime>(builder: MessageDialogBuilder<R>) -> bool {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    builder.show(move |confirmed| {
        let _ = sender.send(confirmed);
    });
    receiver.await.unwrap_or(false)
}
