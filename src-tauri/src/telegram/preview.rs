use base64::Engine;
use grammers_client::media::Media;

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::turso;

use super::drive;

const MAX_PREVIEW_BYTES: usize = 3 * 1024 * 1024;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    /// Miniatura en formato data URL (JPEG).
    pub data_url: String,
    /// image | video
    pub kind: String,
    pub duration: Option<f64>,
    pub resolution: Option<(i32, i32)>,
}

/// Descarga la miniatura de un archivo para mostrarlo en la interfaz.
pub async fn file_preview(state: &AppState, file_id: &str) -> AppResult<Option<Preview>> {
    if let Some(cached) = state.inner.lock().await.previews.get(file_id) {
        return Ok(Some(cached.clone()));
    }

    let db = state.turso_database().await?;
    let record = turso::find_file(&db, file_id)
        .await?
        .ok_or_else(|| AppError::Telegram("El archivo no está en el índice".into()))?;

    let message_ids: Vec<i32> = serde_json::from_str(&record.messages).unwrap_or_default();
    let first = message_ids
        .first()
        .ok_or_else(|| AppError::Telegram("El archivo no tiene partes descargables".into()))?;

    let client = state.get_or_connect().await?;
    let drive = drive::ensure_drive(state, &client).await?;

    let messages = client
        .get_messages_by_id(drive.peer.clone(), &[*first])
        .await
        .map_err(AppError::from)?;
    let message = match messages.into_iter().flatten().next() {
        Some(value) => value,
        None => return Ok(None),
    };

    let media = match message.media() {
        Some(value) => value,
        None => return Ok(None),
    };

    let (thumbs, kind, duration, resolution) = match &media {
        Media::Photo(photo) => (photo.thumbs(), "image", None, None),
        Media::Document(document) => {
            let kind = match document.mime_type() {
                Some(mime) if mime.starts_with("video/") => "video",
                _ => "image",
            };
            (
                document.thumbs(),
                kind,
                document.duration(),
                document.resolution(),
            )
        }
        _ => return Ok(None),
    };

    let thumb = match thumbs.into_iter().max_by_key(|item| item.size()) {
        Some(value) if value.size() > 0 => value,
        _ => return Ok(None),
    };

    let mut bytes: Vec<u8> = Vec::new();
    let mut download = client.iter_download(&thumb);
    while let Some(chunk) = download.next().await.map_err(AppError::from)? {
        if bytes.len() + chunk.len() > MAX_PREVIEW_BYTES {
            break;
        }
        bytes.extend_from_slice(&chunk);
    }

    if bytes.is_empty() {
        return Ok(None);
    }

    let preview = Preview {
        data_url: format!(
            "data:image/jpeg;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        ),
        kind: kind.to_string(),
        duration,
        resolution,
    };

    state
        .inner
        .lock()
        .await
        .previews
        .insert(file_id.to_string(), preview.clone());

    Ok(Some(preview))
}
