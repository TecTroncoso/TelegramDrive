use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Falta configurar las credenciales de Telegram (API ID y API Hash)")]
    NotConfigured,
    #[error("Falta configurar Turso (URL y token de la base de datos)")]
    TursoNotConfigured,
    #[error("Primero debes solicitar un código de acceso")]
    NoLoginToken,
    #[error("Telegram rechazó la operación: {0}")]
    Telegram(String),
    #[error("Turso rechazó la operación: {0}")]
    Turso(String),
    #[error("Error de red o de disco: {0}")]
    Io(String),
}

impl From<libsql::Error> for AppError {
    fn from(value: libsql::Error) -> Self {
        Self::Turso(value.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<grammers_client::InvocationError> for AppError {
    fn from(value: grammers_client::InvocationError) -> Self {
        Self::Telegram(value.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
