use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter};

use crate::error::{AppError, AppResult};
use crate::manifest::Manifest;
use crate::state::AppState;
use crate::turso::{self, FileRecord};

use super::drive;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncReport {
    pub scanned: u32,
    pub imported: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncProgress {
    pub scanned: u32,
    pub imported: u32,
}

/// Reconstruye el índice de Turso leyendo los manifiestos de los captions del canal.
pub async fn sync_drive(state: &AppState, app: AppHandle) -> AppResult<SyncReport> {
    let client = state.get_or_connect().await?;
    let drive = drive::ensure_drive(state, &client).await?;
    let db = state.turso_database().await?;

    let mut parts_by_file: HashMap<String, Vec<(i32, i32, Manifest)>> = HashMap::new();
    let mut scanned: u32 = 0;

    let mut messages = client.iter_messages(drive.peer.clone()).limit(2000);
    while let Some(message) = messages.next().await.map_err(AppError::from)? {
        scanned += 1;
        if let Some(manifest) = Manifest::parse_caption(message.text()) {
            parts_by_file
                .entry(manifest.id.clone())
                .or_default()
                .push((manifest.p, message.id(), manifest));
        }
        if scanned % 100 == 0 {
            let _ = app.emit(
                "sync://progress",
                SyncProgress {
                    scanned,
                    imported: parts_by_file.len() as u32,
                },
            );
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);

    let mut imported: u32 = 0;
    for (file_id, mut entries) in parts_by_file {
        entries.sort_by_key(|(part, _, _)| *part);
        let manifest = entries[0].2.clone();
        let total_parts = manifest.c.max(1);
        let found_parts = entries.len() as i32;
        let status = if found_parts >= total_parts {
            "complete"
        } else {
            "partial"
        };
        let message_ids: Vec<i32> = entries.iter().map(|(_, id, _)| *id).collect();

        turso::upsert_rebuilt(
            &db,
            &FileRecord {
                id: file_id,
                name: manifest.n.clone(),
                folder: manifest.f.clone(),
                size: manifest.s as i64,
                sha256: manifest.h.clone(),
                status: status.into(),
                parts: found_parts as i64,
                messages: serde_json::to_string(&message_ids)
                    .unwrap_or_else(|_| "[]".into()),
                created_at: now,
            },
        )
        .await?;
        imported += 1;
    }

    let _ = app.emit("sync://progress", SyncProgress { scanned, imported });
    Ok(SyncReport { scanned, imported })
}
