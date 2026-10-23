use std::path::PathBuf;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

use crate::error::{AppError, AppResult};
use crate::hash::sha256_file;
use crate::state::AppState;
use crate::turso;

use super::drive;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadEvent {
    pub job_id: String,
    pub file_id: String,
    pub name: String,
    /// downloading | verifying | done | error
    pub phase: String,
    pub sent: u64,
    pub total: u64,
    pub part: i32,
    pub parts: i32,
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadResult {
    pub job_id: String,
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
    pub verified: bool,
}

/// Descarga un archivo de la nube: baja cada parte del canal y las une en el destino.
pub async fn download_file(
    state: &AppState,
    app: AppHandle,
    file_id: String,
    destination: String,
) -> AppResult<DownloadResult> {
    // Una descarga a la vez para no saturar los límites de Telegram.
    let _guard = state.download_lock.lock().await;

    let db = state.turso_database().await?;
    let record = turso::find_file(&db, &file_id)
        .await?
        .ok_or_else(|| AppError::Telegram("El archivo no está en el índice".into()))?;

    if record.status != "complete" {
        return Err(AppError::Telegram(format!(
            "El archivo «{}» está incompleto y no se puede descargar",
            record.name
        )));
    }

    let message_ids: Vec<i32> = serde_json::from_str(&record.messages).unwrap_or_default();
    if message_ids.is_empty() {
        return Err(AppError::Telegram(format!(
            "No hay partes descargables para «{}»",
            record.name
        )));
    }

    let client = state.get_or_connect().await?;
    let drive = drive::ensure_drive(state, &client).await?;

    let job_id = uuid::Uuid::new_v4().to_string();
    let total = record.size.max(0) as u64;
    let parts = message_ids.len() as i32;
    let name = record.name.clone();

    let emit = |phase: &str, sent: u64, part: i32, message: Option<String>| {
        let _ = app.emit(
            "download://progress",
            DownloadEvent {
                job_id: job_id.clone(),
                file_id: file_id.clone(),
                name: name.clone(),
                phase: phase.to_string(),
                sent,
                total,
                part,
                parts,
                message,
            },
        );
    };

    let target = PathBuf::from(&destination);
    if let Some(parent) = target.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }

    emit("downloading", 0, 0, None);

    let mut file = tokio::fs::File::create(&target).await?;
    let mut written: u64 = 0;
    let mut last_emit = Instant::now();

    for (index, message_id) in message_ids.iter().enumerate() {
        let messages = client
            .get_messages_by_id(drive.peer.clone(), &[*message_id])
            .await
            .map_err(AppError::from)?;

        let message = messages
            .into_iter()
            .flatten()
            .next()
            .ok_or_else(|| {
                AppError::Telegram(format!(
                    "No se encontró la parte {} en el canal de la nube",
                    index + 1
                ))
            })?;

        let media = message.media().ok_or_else(|| {
            AppError::Telegram(format!("La parte {} no contiene un archivo", index + 1))
        })?;

        let mut download = client.iter_download(&media);
        while let Some(chunk) = download.next().await.map_err(AppError::from)? {
            file.write_all(&chunk).await?;
            written += chunk.len() as u64;

            if last_emit.elapsed() >= Duration::from_millis(200) {
                emit("downloading", written, index as i32 + 1, None);
                last_emit = Instant::now();
            }
        }

        emit("downloading", written, index as i32 + 1, None);
    }

    file.flush().await?;
    file.sync_all().await?;
    drop(file);

    emit("verifying", written, parts, None);

    let digest = sha256_file(&target).await?;
    let expected = record.sha256.trim().to_lowercase();
    let verified = !expected.is_empty() && digest == expected;

    if !expected.is_empty() && !verified {
        let _ = tokio::fs::remove_file(&target).await;
        emit(
            "error",
            written,
            parts,
            Some("La huella no coincide: el archivo se descartó".into()),
        );
        return Err(AppError::Telegram(
            "La huella del archivo descargado no coincide con la original".into(),
        ));
    }

    emit("done", written, parts, None);

    Ok(DownloadResult {
        job_id,
        path: target.to_string_lossy().into_owned(),
        bytes: written,
        sha256: digest,
        verified,
    })
}
