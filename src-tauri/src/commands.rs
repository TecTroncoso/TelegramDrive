use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::telegram::auth::{self, LoginOutcome, MeInfo, StatusPayload};
use crate::telegram::download::{self, DownloadResult};
use crate::telegram::drive::{self, DriveInfo};
use crate::telegram::preview::{self, Preview};
use crate::telegram::sync::{self, SyncReport};
use crate::telegram::upload::{self, UploadSummary};
use crate::turso::{self, FileEntry};

#[tauri::command]
pub async fn app_status(state: State<'_, AppState>) -> AppResult<StatusPayload> {
    auth::status(&state).await
}

#[tauri::command]
pub async fn save_credentials(
    state: State<'_, AppState>,
    api_id: i32,
    api_hash: String,
) -> AppResult<StatusPayload> {
    auth::save_credentials(&state, api_id, api_hash).await
}

#[tauri::command]
pub async fn request_code(state: State<'_, AppState>, phone: String) -> AppResult<()> {
    auth::request_code(&state, phone).await
}

#[tauri::command]
pub async fn submit_code(state: State<'_, AppState>, code: String) -> AppResult<LoginOutcome> {
    auth::submit_code(&state, code).await
}

#[tauri::command]
pub async fn submit_password(state: State<'_, AppState>, password: String) -> AppResult<MeInfo> {
    auth::submit_password(&state, password).await
}

#[tauri::command]
pub async fn logout(state: State<'_, AppState>) -> AppResult<StatusPayload> {
    auth::logout(&state).await
}

#[tauri::command]
pub async fn save_turso(
    state: State<'_, AppState>,
    url: String,
    token: String,
) -> AppResult<StatusPayload> {
    turso::configure(&state, url, token).await?;
    auth::status(&state).await
}

#[tauri::command]
pub async fn list_files(
    state: State<'_, AppState>,
    view: Option<String>,
    search: Option<String>,
) -> AppResult<Vec<FileEntry>> {
    let db = state.turso_database().await?;
    let view = turso::View::parse(view.as_deref().unwrap_or("drive"));
    turso::list_files(&db, view, search.as_deref()).await
}

#[tauri::command]
pub async fn toggle_favorite(
    state: State<'_, AppState>,
    file_id: String,
) -> AppResult<bool> {
    let db = state.turso_database().await?;
    turso::toggle_favorite(&db, &file_id).await
}

#[tauri::command]
pub async fn trash_file(state: State<'_, AppState>, file_id: String) -> AppResult<()> {
    let db = state.turso_database().await?;
    turso::set_trashed(&db, &file_id, true).await
}

#[tauri::command]
pub async fn restore_file(state: State<'_, AppState>, file_id: String) -> AppResult<()> {
    let db = state.turso_database().await?;
    turso::set_trashed(&db, &file_id, false).await
}

#[tauri::command]
pub async fn purge_file(state: State<'_, AppState>, file_id: String) -> AppResult<()> {
    let db = state.turso_database().await?;

    if let Some(record) = turso::find_file(&db, &file_id).await? {
        let message_ids: Vec<i32> = serde_json::from_str(&record.messages).unwrap_or_default();
        if !message_ids.is_empty() {
            let client = state.get_or_connect().await?;
            let drive = drive::ensure_drive(&state, &client).await?;
            let _ = client
                .delete_messages(drive.peer.clone(), &message_ids)
                .await
                .map_err(AppError::from)?;
        }
    }

    turso::delete_file(&db, &file_id).await
}

#[tauri::command]
pub async fn upload_files(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
) -> AppResult<UploadSummary> {
    upload::upload_paths(&state, app, paths).await
}

#[tauri::command]
pub async fn sync_drive(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<SyncReport> {
    sync::sync_drive(&state, app).await
}

#[tauri::command]
pub async fn drive_info(state: State<'_, AppState>) -> AppResult<DriveInfo> {
    let client = state.get_or_connect().await?;
    let drive = drive::ensure_drive(&state, &client).await?;
    let premium = drive::is_premium(&state, &client).await;

    Ok(DriveInfo {
        chat_id: drive.chat_id,
        title: drive.title,
        premium,
        part_limit_bytes: drive::part_limit_bytes(premium),
    })
}

#[tauri::command]
pub async fn download_file(
    app: AppHandle,
    state: State<'_, AppState>,
    file_id: String,
    path: String,
) -> AppResult<DownloadResult> {
    download::download_file(&state, app, file_id, path).await
}

#[tauri::command]
pub async fn reveal_path(app: AppHandle, path: String) -> AppResult<()> {
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|err| AppError::Io(err.to_string()))
}

#[tauri::command]
pub async fn file_preview(
    state: State<'_, AppState>,
    file_id: String,
) -> AppResult<Option<Preview>> {
    preview::file_preview(&state, &file_id).await
}
