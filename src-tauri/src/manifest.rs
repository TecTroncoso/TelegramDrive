use serde::{Deserialize, Serialize};

/// Marcador que identifica los mensajes que pertenecen a la nube.
pub const CAPTION_PREFIX: &str = "td1|";

/// Manifiesto compacto que viaja en el caption de cada parte (< 1024 caracteres).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    /// Versión del formato.
    pub v: u8,
    /// Id lógico del archivo (uuid).
    pub id: String,
    /// Nombre original (puede venir truncado si era enorme).
    pub n: String,
    /// Tamaño total en bytes.
    pub s: u64,
    /// Número de parte (1-based).
    pub p: i32,
    /// Total de partes.
    pub c: i32,
    /// SHA-256 hexadecimal del archivo completo.
    pub h: String,
    /// Carpeta virtual.
    pub f: String,
}

impl Manifest {
    pub fn to_caption(&self) -> String {
        let raw = serde_json::to_string(self).unwrap_or_default();
        format!("{CAPTION_PREFIX}{raw}")
    }

    pub fn parse_caption(text: &str) -> Option<Self> {
        text.strip_prefix(CAPTION_PREFIX)
            .and_then(|raw| serde_json::from_str(raw).ok())
    }
}

/// Los captions de Telegram admiten ~1024 caracteres; deja margen para el JSON.
pub fn truncate_name(name: &str) -> String {
    const MAX_CHARS: usize = 600;
    if name.chars().count() <= MAX_CHARS {
        name.to_string()
    } else {
        name.chars().take(MAX_CHARS).collect()
    }
}
