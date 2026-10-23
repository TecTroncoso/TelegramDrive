use std::path::PathBuf;
use std::sync::Arc;

use grammers_client::client::{LoginToken, PasswordToken};
use grammers_client::{Client, SenderPool};
use grammers_session::storages::SqliteSession;
use libsql::Database;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub api_id: i32,
    pub api_hash: String,
    pub turso_url: Option<String>,
    pub turso_token: Option<String>,
    pub drive_chat_id: Option<i64>,
}

impl AppConfig {
    pub fn has_api(&self) -> bool {
        self.api_id > 0 && !self.api_hash.is_empty()
    }

    pub fn turso(&self) -> Option<(String, String)> {
        match (&self.turso_url, &self.turso_token) {
            (Some(url), Some(token)) if !url.is_empty() && !token.is_empty() => {
                Some((url.clone(), token.clone()))
            }
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Inner {
    pub config: AppConfig,
    pub client: Option<Client>,
    pub login_token: Option<LoginToken>,
    pub password_token: Option<PasswordToken>,
    pub drive: Option<crate::telegram::drive::Drive>,
    pub premium: Option<bool>,
    pub previews: std::collections::HashMap<String, crate::telegram::preview::Preview>,
}

/// Rellena con el `.env` los datos que todavía no estén guardados en `config.json`.
/// Lo guardado desde la app tiene prioridad: el `.env` solo sirve de arranque.
fn apply_env_defaults(config: &mut AppConfig) {
    let values = crate::envfile::load();
    if values.is_empty() {
        return;
    }

    if !config.has_api() {
        if let Some(api_id) = crate::envfile::get(&values, "API_ID")
            .and_then(|value| value.parse::<i32>().ok())
        {
            config.api_id = api_id;
        }
        if let Some(api_hash) = crate::envfile::get(&values, "API_HASH") {
            config.api_hash = api_hash;
        }
    }

    if config.turso().is_none() {
        if let Some(url) = crate::envfile::get(&values, "TURSO_URL") {
            config.turso_url = Some(url);
        }
        if let Some(token) = crate::envfile::get(&values, "TURSO_TOKEN") {
            config.turso_token = Some(token);
        }
    }
}

pub struct AppState {
    pub data_dir: PathBuf,
    pub inner: tokio::sync::Mutex<Inner>,
    pub turso: tokio::sync::Mutex<Option<Arc<Database>>>,
    pub upload_lock: tokio::sync::Mutex<()>,
    pub download_lock: tokio::sync::Mutex<()>,
}

impl AppState {
    pub fn new(data_dir: PathBuf) -> Self {
        let mut config = std::fs::read_to_string(data_dir.join("config.json"))
            .ok()
            .and_then(|raw| serde_json::from_str::<AppConfig>(&raw).ok())
            .unwrap_or_default();

        apply_env_defaults(&mut config);

        Self {
            data_dir,
            inner: tokio::sync::Mutex::new(Inner {
                config,
                ..Default::default()
            }),
            turso: tokio::sync::Mutex::new(None),
            upload_lock: tokio::sync::Mutex::new(()),
            download_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub fn session_path(&self) -> PathBuf {
        self.data_dir.join("session.sqlite")
    }

    pub fn persist_config(&self, config: &AppConfig) -> AppResult<()> {
        let raw =
            serde_json::to_string_pretty(config).map_err(|err| AppError::Io(err.to_string()))?;
        std::fs::write(self.data_dir.join("config.json"), raw)?;
        Ok(())
    }

    pub async fn config_snapshot(&self) -> AppConfig {
        self.inner.lock().await.config.clone()
    }

    pub async fn update_config<F: FnOnce(&mut AppConfig)>(&self, update: F) -> AppResult<AppConfig> {
        let mut inner = self.inner.lock().await;
        update(&mut inner.config);
        let snapshot = inner.config.clone();
        drop(inner);
        self.persist_config(&snapshot)?;
        Ok(snapshot)
    }

    /// Devuelve el cliente conectado, creando la sesión y conectando la primera vez.
    pub async fn get_or_connect(&self) -> AppResult<Client> {
        let mut inner = self.inner.lock().await;

        if let Some(client) = &inner.client {
            return Ok(client.clone());
        }
        if !inner.config.has_api() {
            return Err(AppError::NotConfigured);
        }

        let api_id = inner.config.api_id;
        let path = self.session_path().to_string_lossy().into_owned();
        let session = Arc::new(
            SqliteSession::open(path.as_str())
                .await
                .map_err(|err| AppError::Io(err.to_string()))?,
        );

        let SenderPool { runner, handle, .. } = SenderPool::new(session, api_id);
        let client = Client::new(handle);
        tauri::async_runtime::spawn(runner.run());

        inner.client = Some(client.clone());
        Ok(client)
    }

    /// Devuelve la base de Turso, conectándose la primera vez.
    pub async fn turso_database(&self) -> AppResult<Arc<Database>> {
        let mut guard = self.turso.lock().await;
        if let Some(db) = guard.as_ref() {
            return Ok(Arc::clone(db));
        }

        let (url, token) = self
            .inner
            .lock()
            .await
            .config
            .turso()
            .ok_or(AppError::TursoNotConfigured)?;

        let db = crate::turso::connect(&url, &token).await?;
        crate::turso::migrate(&db).await?;

        let db = Arc::new(db);
        *guard = Some(Arc::clone(&db));
        Ok(db)
    }

    pub async fn set_turso(&self, db: Database) {
        *self.turso.lock().await = Some(Arc::new(db));
    }

    /// Drops the in-memory client and any pending login state.
    pub async fn disconnect(&self) {
        let mut inner = self.inner.lock().await;
        inner.client = None;
        inner.login_token = None;
        inner.password_token = None;
        inner.drive = None;
        inner.premium = None;
    }
}
