use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use grammers_client::message::InputMessage;
use grammers_client::Client;
use tauri::{AppHandle, Emitter};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt, ReadBuf, SeekFrom};

use crate::error::{AppError, AppResult};
use crate::hash::sha256_file;
use crate::manifest::{truncate_name, Manifest};
use crate::state::AppState;
use crate::turso::{self, FileRecord};

use super::drive::{self, Drive};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadEvent {
    pub job_id: String,
    pub file_id: String,
    pub name: String,
    /// hashing | uploading | done | skipped | error
    pub phase: String,
    pub sent: u64,
    pub total: u64,
    pub part: i32,
    pub parts: i32,
    pub message: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadSummary {
    pub done: u32,
    pub skipped: u32,
    pub failed: u32,
}

enum Outcome {
    Done,
    Skipped,
}

pub async fn upload_paths(
    state: &AppState,
    app: AppHandle,
    paths: Vec<String>,
) -> AppResult<UploadSummary> {
    // Una cola a la vez para respetar los límites de Telegram (FLOOD_WAIT).
    let _guard = state.upload_lock.lock().await;

    let client = state.get_or_connect().await?;
    let drive = drive::ensure_drive(state, &client).await?;
    let premium = drive::is_premium(state, &client).await;
    let limit = drive::part_limit_bytes(premium);
    let db = state.turso_database().await?;

    let mut summary = UploadSummary {
        done: 0,
        skipped: 0,
        failed: 0,
    };

    for path in paths {
        match upload_one(&app, &client, &drive, &db, limit, &path).await {
            Ok(Outcome::Done) => summary.done += 1,
            Ok(Outcome::Skipped) => summary.skipped += 1,
            Err(_) => summary.failed += 1,
        }
    }

    Ok(summary)
}

async fn upload_one(
    app: &AppHandle,
    client: &Client,
    drive: &Drive,
    db: &libsql::Database,
    limit: i64,
    path_str: &str,
) -> AppResult<Outcome> {
    let path = PathBuf::from(path_str);
    let name = path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .ok_or_else(|| AppError::Telegram("Ruta de archivo inválida".into()))?;

    let metadata = tokio::fs::metadata(&path).await?;
    if !metadata.is_file() {
        return Err(AppError::Telegram(format!("No es un archivo: {name}")));
    }
    let size = metadata.len();
    if size == 0 {
        return Err(AppError::Telegram(format!(
            "«{name}» está vacío y no se puede subir"
        )));
    }

    let job_id = uuid::Uuid::new_v4().to_string();
    let file_id = uuid::Uuid::new_v4().to_string();

    let emit = |phase: &str, sent: u64, part: i32, parts: i32, message: Option<String>| {
        let _ = app.emit(
            "upload://progress",
            UploadEvent {
                job_id: job_id.clone(),
                file_id: file_id.clone(),
                name: name.clone(),
                phase: phase.to_string(),
                sent,
                total: size,
                part,
                parts,
                message,
            },
        );
    };

    // 1) Hash local (para deduplicar y verificar).
    emit("hashing", 0, 0, 0, None);
    let sha256 = match sha256_file(&path).await {
        Ok(value) => value,
        Err(err) => {
            emit("error", 0, 0, 0, Some(err.to_string()));
            return Err(err);
        }
    };

    if turso::find_complete_by_sha(db, &sha256).await.unwrap_or(false) {
        emit(
            "skipped",
            size,
            0,
            0,
            Some("Ya está en tu nube (mismo contenido)".into()),
        );
        return Ok(Outcome::Skipped);
    }

    // 2) Troceo y subida parte por parte.
    let parts = if size == 0 {
        1
    } else {
        ((size as i64 + limit - 1) / limit) as i32
    };

    let mut message_ids: Vec<i32> = Vec::with_capacity(parts as usize);

    for index in 0..parts {
        let offset = index as i64 * limit;
        let length = (size as i64 - offset).clamp(0, limit) as u64;

        let mut file = tokio::fs::File::open(&path).await?;
        file.seek(SeekFrom::Start(offset as u64)).await?;
        let limited = file.take(length);

        let counter = Arc::new(AtomicU64::new(0));
        let finished = Arc::new(AtomicBool::new(false));
        let mut stream = ProgressReader {
            inner: limited,
            counter: Arc::clone(&counter),
        };

        let part_name = if parts > 1 {
            format!("{name}.part{:03}", index + 1)
        } else {
            name.clone()
        };

        // Emisor de progreso: lee el contador mientras sube.
        let ticker = {
            let app = app.clone();
            let counter = Arc::clone(&counter);
            let finished = Arc::clone(&finished);
            let job_id = job_id.clone();
            let file_id = file_id.clone();
            let name = name.clone();
            let part = index + 1;
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_millis(350)).await;
                    if finished.load(Ordering::Relaxed) {
                        break;
                    }
                    let sent =
                        offset as u64 + counter.load(Ordering::Relaxed).min(length);
                    let _ = app.emit(
                        "upload://progress",
                        UploadEvent {
                            job_id: job_id.clone(),
                            file_id: file_id.clone(),
                            name: name.clone(),
                            phase: "uploading".into(),
                            sent,
                            total: size,
                            part,
                            parts,
                            message: None,
                        },
                    );
                }
            })
        };

        let uploaded = client
            .upload_stream(&mut stream, length as usize, part_name)
            .await;
        finished.store(true, Ordering::Relaxed);
        let _ = ticker.await;

        let uploaded = match uploaded {
            Ok(value) => value,
            Err(err) => {
                let error = AppError::Io(err.to_string());
                emit("error", offset as u64, index + 1, parts, Some(error.to_string()));
                return Err(error);
            }
        };

        let manifest = Manifest {
            v: 1,
            id: file_id.clone(),
            n: truncate_name(&name),
            s: size,
            p: index + 1,
            c: parts,
            h: sha256.clone(),
            f: "/".into(),
        };

        let message = match client
            .send_message(
                drive.peer.clone(),
                InputMessage::new().text(manifest.to_caption()).document(uploaded),
            )
            .await
        {
            Ok(message) => message,
            Err(err) => {
                let error = AppError::from(err);
                emit("error", offset as u64, index + 1, parts, Some(error.to_string()));
                return Err(error);
            }
        };

        message_ids.push(message.id());
        emit(
            "uploading",
            offset as u64 + length,
            index + 1,
            parts,
            None,
        );
    }

    // 3) Índice en Turso.
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    turso::upsert_file(
        db,
        &FileRecord {
            id: file_id.clone(),
            name: name.clone(),
            folder: "/".into(),
            size: size as i64,
            sha256: sha256.clone(),
            status: "complete".into(),
            parts: parts as i64,
            messages: serde_json::to_string(&message_ids)
                .map_err(|err| AppError::Io(err.to_string()))?,
            created_at: now,
        },
    )
    .await?;

    emit("done", size, parts, parts, None);
    Ok(Outcome::Done)
}

/// Contador de bytes leídos que permite emitir progreso durante la subida.
struct ProgressReader<R> {
    inner: R,
    counter: Arc<AtomicU64>,
}

impl<R: AsyncRead + Unpin> AsyncRead for ProgressReader<R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let this = self.get_mut();
        match Pin::new(&mut this.inner).poll_read(cx, buf) {
            Poll::Ready(Ok(())) => {
                let read = buf.filled().len() - before;
                this.counter.fetch_add(read as u64, Ordering::Relaxed);
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}
