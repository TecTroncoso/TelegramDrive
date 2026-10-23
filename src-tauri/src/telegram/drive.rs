use grammers_client::peer::Peer;
use grammers_client::tl;
use grammers_client::Client;
use grammers_session::types::{PeerAuth, PeerId, PeerRef};

use crate::error::{AppError, AppResult};
use crate::state::AppState;
use crate::turso;

pub const DRIVE_TITLE: &str = "Mi Nube";
const DRIVE_ABOUT: &str =
    "Nube privada creada por TeleDrive (#teledrive). No borres ni renombres este canal: tus archivos viven aquí.";

/// Referencia al canal que hace de nube.
#[derive(Debug, Clone)]
pub struct Drive {
    pub peer: PeerRef,
    pub chat_id: i64,
    pub title: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    pub chat_id: i64,
    pub title: String,
    pub premium: bool,
    pub part_limit_bytes: i64,
}

pub const MB: i64 = 1024 * 1024;

/// Busca el canal de la nube (por id guardado o por título) y lo crea si no existe.
pub async fn ensure_drive(state: &AppState, client: &Client) -> AppResult<Drive> {
    if let Some(drive) = state.inner.lock().await.drive.clone() {
        return Ok(drive);
    }

    let mut target_id = state.config_snapshot().await.drive_chat_id;
    if target_id.is_none() {
        if let Ok(db) = state.turso_database().await {
            if let Ok(Some(value)) = turso::get_setting(&db, "drive_chat_id").await {
                target_id = value.parse::<i64>().ok();
            }
        }
    }

    let mut by_title: Option<Drive> = None;
    let mut dialogs = client.iter_dialogs();
    while let Some(dialog) = dialogs.next().await.map_err(AppError::from)? {
        let peer = dialog.peer();
        if let Peer::Channel(channel) = peer {
            let bare_id = channel.id().bare_id();
            let title = channel.title().to_string();
            let drive = Drive {
                peer: dialog.peer_ref(),
                chat_id: bare_id.unwrap_or_default(),
                title,
            };

            if target_id.is_some() && bare_id == target_id {
                return remember_drive(state, drive).await;
            }
            if by_title.is_none() && drive.title == DRIVE_TITLE {
                by_title = Some(drive);
            }
        }
    }

    if let Some(drive) = by_title {
        return remember_drive(state, drive).await;
    }

    // No existe todavía: lo creamos como canal privado de difusión.
    let updates = client
        .invoke(&tl::functions::channels::CreateChannel {
            broadcast: true,
            megagroup: false,
            for_import: false,
            forum: false,
            title: DRIVE_TITLE.into(),
            about: DRIVE_ABOUT.into(),
            geo_point: None,
            address: None,
            ttl_period: None,
        })
        .await
        .map_err(AppError::from)?;

    let (chat_id, access_hash) = extract_created_channel(&updates)
        .ok_or_else(|| AppError::Telegram("No se pudo leer el canal recién creado".into()))?;

    let id = PeerId::channel(chat_id)
        .ok_or_else(|| AppError::Telegram("Identificador de canal inválido".into()))?;

    let drive = Drive {
        peer: PeerRef {
            id,
            auth: PeerAuth::from_hash(access_hash),
        },
        chat_id,
        title: DRIVE_TITLE.to_string(),
    };

    remember_drive(state, drive).await
}

/// Detecta si la cuenta es Premium (límite de 4 GB por archivo en lugar de 2 GB).
pub async fn is_premium(state: &AppState, client: &Client) -> bool {
    if let Some(premium) = state.inner.lock().await.premium {
        return premium;
    }

    let premium = match client
        .invoke(&tl::functions::users::GetFullUser {
            id: tl::enums::InputUser::UserSelf,
        })
        .await
    {
        Ok(tl::enums::users::UserFull::Full(full)) => full
            .users
            .iter()
            .find_map(|user| match user {
                tl::enums::User::User(user) => Some(user.premium),
                _ => None,
            })
            .unwrap_or(false),
        _ => false,
    };

    state.inner.lock().await.premium = Some(premium);
    premium
}

pub fn part_limit_bytes(premium: bool) -> i64 {
    if premium {
        3990 * MB
    } else {
        1990 * MB
    }
}

fn extract_created_channel(updates: &tl::enums::Updates) -> Option<(i64, i64)> {
    let chats = match updates {
        tl::enums::Updates::Updates(payload) => &payload.chats,
        tl::enums::Updates::Combined(payload) => &payload.chats,
        _ => return None,
    };

    for chat in chats {
        if let tl::enums::Chat::Channel(channel) = chat {
            if let Some(hash) = channel.access_hash {
                return Some((channel.id, hash));
            }
        }
    }
    None
}

async fn remember_drive(state: &AppState, drive: Drive) -> AppResult<Drive> {
    state.inner.lock().await.drive = Some(drive.clone());

    let chat_id = drive.chat_id;
    let _ = state
        .update_config(|config| config.drive_chat_id = Some(chat_id))
        .await;

    if let Ok(db) = state.turso_database().await {
        let _ = turso::set_setting(&db, "drive_chat_id", &chat_id.to_string()).await;
    }

    Ok(drive)
}
