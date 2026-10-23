use grammers_client::{peer::User, SignInError};
use serde::Serialize;

use crate::error::{AppError, AppResult};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeInfo {
    pub id: i64,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub phone: Option<String>,
}

impl From<&User> for MeInfo {
    fn from(user: &User) -> Self {
        Self {
            id: user.id().bare_id().unwrap_or_default(),
            first_name: user.first_name().map(str::to_owned),
            last_name: user.last_name().map(str::to_owned),
            username: user.username().map(str::to_owned),
            phone: user.phone().map(str::to_owned),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub configured: bool,
    pub authorized: bool,
    pub me: Option<MeInfo>,
    pub turso_configured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginOutcome {
    pub status: String,
    pub me: Option<MeInfo>,
    pub hint: Option<String>,
}

pub async fn status(state: &AppState) -> AppResult<StatusPayload> {
    let config = state.config_snapshot().await;
    let turso_configured = config.turso().is_some();

    if !config.has_api() {
        return Ok(StatusPayload {
            configured: false,
            authorized: false,
            me: None,
            turso_configured,
        });
    }

    let client = state.get_or_connect().await?;
    let authorized = client.is_authorized().await.map_err(AppError::from)?;

    let me = if authorized {
        let user = client.get_me().await.map_err(AppError::from)?;
        Some(MeInfo::from(&user))
    } else {
        None
    };

    Ok(StatusPayload {
        configured: true,
        authorized,
        me,
        turso_configured,
    })
}

pub async fn save_credentials(
    state: &AppState,
    api_id: i32,
    api_hash: String,
) -> AppResult<StatusPayload> {
    let api_hash = api_hash.trim().to_string();
    if api_id <= 0 || api_hash.is_empty() {
        return Err(AppError::Telegram(
            "API ID o API Hash con formato inválido".into(),
        ));
    }

    let changed = {
        let config = state.config_snapshot().await;
        config.api_id != api_id || config.api_hash != api_hash
    };

    if changed {
        state.disconnect().await;
        std::fs::remove_file(state.session_path()).ok();
    }

    state
        .update_config(|config| {
            config.api_id = api_id;
            config.api_hash = api_hash.clone();
        })
        .await?;

    status(state).await
}

pub async fn request_code(state: &AppState, phone: String) -> AppResult<()> {
    let client = state.get_or_connect().await?;
    let api_hash = state.config_snapshot().await.api_hash;

    let token = client
        .request_login_code(phone.trim(), &api_hash)
        .await
        .map_err(AppError::from)?;

    state.inner.lock().await.login_token = Some(token);
    Ok(())
}

pub async fn submit_code(state: &AppState, code: String) -> AppResult<LoginOutcome> {
    let client = state.get_or_connect().await?;
    let token = {
        let mut inner = state.inner.lock().await;
        inner.login_token.take().ok_or(AppError::NoLoginToken)?
    };

    match client.sign_in(&token, code.trim()).await {
        Ok(user) => {
            state.inner.lock().await.login_token = None;
            Ok(LoginOutcome {
                status: "ok".into(),
                me: Some(MeInfo::from(&user)),
                hint: None,
            })
        }
        Err(SignInError::PasswordRequired(password_token)) => {
            let hint = password_token.hint().map(str::to_owned);
            state.inner.lock().await.password_token = Some(password_token);
            Ok(LoginOutcome {
                status: "passwordRequired".into(),
                me: None,
                hint,
            })
        }
        Err(err) => {
            // Keep the token so the user can retry with the correct code.
            state.inner.lock().await.login_token = Some(token);
            Err(AppError::Telegram(err.to_string()))
        }
    }
}

pub async fn submit_password(state: &AppState, password: String) -> AppResult<MeInfo> {
    let client = state.get_or_connect().await?;
    let token = {
        let mut inner = state.inner.lock().await;
        inner.password_token.take().ok_or(AppError::NoLoginToken)?
    };

    match client.check_password(token, password).await {
        Ok(user) => Ok(MeInfo::from(&user)),
        Err(SignInError::PasswordRequired(password_token)) => {
            let hint = password_token.hint().map(str::to_owned);
            state.inner.lock().await.password_token = Some(password_token);
            let suffix = hint.map(|h| format!(" (pista: {h})")).unwrap_or_default();
            Err(AppError::Telegram(format!("Contraseña incorrecta{suffix}")))
        }
        Err(err) => Err(AppError::Telegram(err.to_string())),
    }
}

pub async fn logout(state: &AppState) -> AppResult<StatusPayload> {
    if let Ok(client) = state.get_or_connect().await {
        client.sign_out().await.map_err(AppError::from)?;
    }

    state.disconnect().await;
    std::fs::remove_file(state.session_path()).ok();

    status(state).await
}
