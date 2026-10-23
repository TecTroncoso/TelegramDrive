use libsql::{params, Builder, Database, Value};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub size: i64,
    pub status: String,
    pub parts: i64,
    pub created_at: i64,
    pub favorite: bool,
    pub trashed: bool,
}

pub struct FileRecord {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub size: i64,
    pub sha256: String,
    pub status: String,
    pub parts: i64,
    pub messages: String,
    pub created_at: i64,
}

/// Archivo del índice con los ids de mensaje de cada parte (para descargar).
#[derive(Debug, Clone)]
pub struct StoredFile {
    pub name: String,
    pub size: i64,
    pub sha256: String,
    pub status: String,
    pub messages: String,
}

fn turso_err(err: libsql::Error) -> AppError {
    AppError::Turso(err.to_string())
}

pub async fn connect(url: &str, token: &str) -> AppResult<Database> {
    Builder::new_remote(url.to_string(), token.to_string())
        .build()
        .await
        .map_err(turso_err)
}

/// Crea las tablas del índice si no existen.
pub async fn migrate(db: &Database) -> AppResult<()> {
    let conn = db.connect().map_err(turso_err)?;
    for statement in [
        "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS files (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            folder TEXT NOT NULL DEFAULT '/',
            size INTEGER NOT NULL DEFAULT 0,
            sha256 TEXT,
            status TEXT NOT NULL DEFAULT 'complete',
            parts INTEGER NOT NULL DEFAULT 1,
            messages TEXT,
            created_at INTEGER NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0
        )",
        "CREATE INDEX IF NOT EXISTS idx_files_folder ON files(folder)",
        "CREATE INDEX IF NOT EXISTS idx_files_sha ON files(sha256)",
    ] {
        conn.execute(statement, ()).await.map_err(turso_err)?;
    }

    // Columnas añadidas después de la primera versión del índice.
    let columns = table_columns(&conn).await?;
    if !columns.iter().any(|name| name == "favorite") {
        conn.execute(
            "ALTER TABLE files ADD COLUMN favorite INTEGER NOT NULL DEFAULT 0",
            (),
        )
        .await
        .map_err(turso_err)?;
    }
    if !columns.iter().any(|name| name == "trashed") {
        conn.execute(
            "ALTER TABLE files ADD COLUMN trashed INTEGER NOT NULL DEFAULT 0",
            (),
        )
        .await
        .map_err(turso_err)?;
    }
    for statement in [
        "CREATE INDEX IF NOT EXISTS idx_files_favorite ON files(favorite)",
        "CREATE INDEX IF NOT EXISTS idx_files_trashed ON files(trashed)",
    ] {
        conn.execute(statement, ()).await.map_err(turso_err)?;
    }
    Ok(())
}

async fn table_columns(conn: &libsql::Connection) -> AppResult<Vec<String>> {
    let mut rows = conn
        .query("PRAGMA table_info(files)", ())
        .await
        .map_err(turso_err)?;
    let mut columns = Vec::new();
    while let Some(row) = rows.next().await.map_err(turso_err)? {
        columns.push(row.get::<String>(1).map_err(turso_err)?);
    }
    Ok(columns)
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

/// Valida las credenciales y prepara el esquema.
pub async fn configure(state: &AppState, url: String, token: String) -> AppResult<()> {
    let url = url.trim().to_string();
    let token = token.trim().to_string();
    if url.is_empty() || token.is_empty() {
        return Err(AppError::Turso(
            "La URL y el token de Turso son obligatorios".into(),
        ));
    }

    let db = connect(&url, &token).await?;
    migrate(&db).await?;

    state
        .update_config(|config| {
            config.turso_url = Some(url);
            config.turso_token = Some(token);
        })
        .await?;
    state.set_turso(db).await;
    Ok(())
}

pub async fn get_setting(db: &Database, key: &str) -> AppResult<Option<String>> {
    let conn = db.connect().map_err(turso_err)?;
    let mut rows = conn
        .query("SELECT value FROM settings WHERE key = ?1", params![key])
        .await
        .map_err(turso_err)?;
    match rows.next().await.map_err(turso_err)? {
        Some(row) => Ok(Some(row.get::<String>(0).map_err(turso_err)?)),
        None => Ok(None),
    }
}

pub async fn set_setting(db: &Database, key: &str, value: &str) -> AppResult<()> {
    let conn = db.connect().map_err(turso_err)?;
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .await
    .map_err(turso_err)?;
    Ok(())
}

pub async fn find_complete_by_sha(db: &Database, sha256: &str) -> AppResult<bool> {
    let conn = db.connect().map_err(turso_err)?;
    let mut rows = conn
        .query(
            "SELECT id FROM files WHERE sha256 = ?1 AND status = 'complete' LIMIT 1",
            params![sha256],
        )
        .await
        .map_err(turso_err)?;
    Ok(rows.next().await.map_err(turso_err)?.is_some())
}

pub async fn upsert_file(db: &Database, record: &FileRecord) -> AppResult<()> {
    let conn = db.connect().map_err(turso_err)?;
    conn.execute(
        "INSERT INTO files (id, name, folder, size, sha256, status, parts, messages, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT(id) DO UPDATE SET
            name = excluded.name,
            folder = excluded.folder,
            size = excluded.size,
            sha256 = excluded.sha256,
            status = excluded.status,
            parts = excluded.parts,
            messages = excluded.messages,
            updated_at = excluded.updated_at",
        params![
            record.id.as_str(),
            record.name.as_str(),
            record.folder.as_str(),
            record.size,
            record.sha256.as_str(),
            record.status.as_str(),
            record.parts,
            record.messages.as_str(),
            record.created_at,
        ],
    )
    .await
    .map_err(turso_err)?;
    Ok(())
}

/// Upsert usado al reconstruir el índice desde los captions del canal.
/// No pisa los datos de un archivo que ya estaba completo.
pub async fn upsert_rebuilt(db: &Database, record: &FileRecord) -> AppResult<()> {
    let conn = db.connect().map_err(turso_err)?;
    conn.execute(
        "INSERT INTO files (id, name, folder, size, sha256, status, parts, messages, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)
         ON CONFLICT(id) DO UPDATE SET
            messages = excluded.messages,
            parts = MAX(excluded.parts, files.parts),
            status = CASE WHEN files.status = 'complete' THEN 'complete' ELSE excluded.status END,
            sha256 = COALESCE(files.sha256, excluded.sha256),
            updated_at = excluded.updated_at",
        params![
            record.id.as_str(),
            record.name.as_str(),
            record.folder.as_str(),
            record.size,
            record.sha256.as_str(),
            record.status.as_str(),
            record.parts,
            record.messages.as_str(),
            record.created_at,
        ],
    )
    .await
    .map_err(turso_err)?;
    Ok(())
}

pub async fn find_file(db: &Database, id: &str) -> AppResult<Option<StoredFile>> {
    let conn = db.connect().map_err(turso_err)?;
    let mut rows = conn
        .query(
            "SELECT name, size, sha256, status, messages
             FROM files
             WHERE id = ?1
             LIMIT 1",
            params![id],
        )
        .await
        .map_err(turso_err)?;

    match rows.next().await.map_err(turso_err)? {
        Some(row) => Ok(Some(StoredFile {
            name: row.get(0).map_err(turso_err)?,
            size: row.get(1).map_err(turso_err)?,
            sha256: row
                .get::<Option<String>>(2)
                .map_err(turso_err)?
                .unwrap_or_default(),
            status: row.get(3).map_err(turso_err)?,
            messages: row
                .get::<Option<String>>(4)
                .map_err(turso_err)?
                .unwrap_or_default(),
        })),
        None => Ok(None),
    }
}

/// Vistas disponibles del índice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Drive,
    Recent,
    Favorites,
    Trash,
}

impl View {
    pub fn parse(value: &str) -> View {
        match value {
            "recent" => View::Recent,
            "favorites" => View::Favorites,
            "trash" => View::Trash,
            _ => View::Drive,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            View::Drive => "drive",
            View::Recent => "recent",
            View::Favorites => "favorites",
            View::Trash => "trash",
        }
    }
}

pub async fn list_files(
    db: &Database,
    view: View,
    search: Option<&str>,
) -> AppResult<Vec<FileEntry>> {
    let conn = db.connect().map_err(turso_err)?;
    let search_param = match search.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => Value::Text(value.to_lowercase()),
        None => Value::Null,
    };
    let since = unix_now() - 7 * 24 * 60 * 60;

    let mut rows = conn
        .query(
            "SELECT id, name, folder, size, status, parts, created_at, favorite, trashed
             FROM files
             WHERE (
                    (?1 = 'drive' AND trashed = 0)
                 OR (?1 = 'recent' AND trashed = 0 AND created_at >= ?3)
                 OR (?1 = 'favorites' AND favorite = 1 AND trashed = 0)
                 OR (?1 = 'trash' AND trashed = 1)
             )
             AND (?2 IS NULL OR lower(name) LIKE '%' || ?2 || '%')
             ORDER BY created_at DESC
             LIMIT 500",
            params![view.as_str(), search_param, since],
        )
        .await
        .map_err(turso_err)?;

    let mut files = Vec::new();
    while let Some(row) = rows.next().await.map_err(turso_err)? {
        files.push(FileEntry {
            id: row.get(0).map_err(turso_err)?,
            name: row.get(1).map_err(turso_err)?,
            folder: row.get(2).map_err(turso_err)?,
            size: row.get(3).map_err(turso_err)?,
            status: row.get(4).map_err(turso_err)?,
            parts: row.get(5).map_err(turso_err)?,
            created_at: row.get(6).map_err(turso_err)?,
            favorite: row.get(7).map_err(turso_err)?,
            trashed: row.get(8).map_err(turso_err)?,
        });
    }
    Ok(files)
}

/// Marca o desmarca un archivo como favorito y devuelve el nuevo estado.
pub async fn toggle_favorite(db: &Database, id: &str) -> AppResult<bool> {
    let conn = db.connect().map_err(turso_err)?;
    conn.execute(
        "UPDATE files SET favorite = 1 - favorite, updated_at = ?2 WHERE id = ?1",
        params![id, unix_now()],
    )
    .await
    .map_err(turso_err)?;

    let mut rows = conn
        .query("SELECT favorite FROM files WHERE id = ?1", params![id])
        .await
        .map_err(turso_err)?;
    match rows.next().await.map_err(turso_err)? {
        Some(row) => Ok(row.get::<bool>(0).map_err(turso_err)?),
        None => Ok(false),
    }
}

/// Envía un archivo a la papelera o lo restaura.
pub async fn set_trashed(db: &Database, id: &str, trashed: bool) -> AppResult<()> {
    let conn = db.connect().map_err(turso_err)?;
    conn.execute(
        "UPDATE files SET trashed = ?2, updated_at = ?3 WHERE id = ?1",
        params![id, trashed as i64, unix_now()],
    )
    .await
    .map_err(turso_err)?;
    Ok(())
}

/// Elimina el archivo del índice (los mensajes de Telegram se borran aparte).
pub async fn delete_file(db: &Database, id: &str) -> AppResult<()> {
    let conn = db.connect().map_err(turso_err)?;
    conn.execute("DELETE FROM files WHERE id = ?1", params![id])
        .await
        .map_err(turso_err)?;
    Ok(())
}
