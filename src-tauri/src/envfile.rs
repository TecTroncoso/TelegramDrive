use std::collections::HashMap;
use std::path::PathBuf;

/// Lee un archivo `.env` con formato `CLAVE=valor` (comentarios con `#`, comillas opcionales).
pub fn parse(contents: &str) -> HashMap<String, String> {
    let mut values = HashMap::new();

    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        if key.is_empty() {
            continue;
        }

        let mut value = value.trim().to_string();
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = value[1..value.len() - 1].to_string();
        }

        values.insert(key.to_string(), value);
    }

    values
}

/// Busca el `.env`: primero junto al ejecutable, luego en el directorio de trabajo.
pub fn locate() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join(".env"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join(".env"));
    }

    candidates.into_iter().find(|path| path.is_file())
}

/// Carga las variables del `.env` encontrado (vacío si no existe).
pub fn load() -> HashMap<String, String> {
    locate()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|contents| parse(&contents))
        .unwrap_or_default()
}

/// Busca una clave aceptando el nombre con y sin prefijo `TELEDRIVE_`.
pub fn get(values: &HashMap<String, String>, key: &str) -> Option<String> {
    values
        .get(&format!("TELEDRIVE_{key}"))
        .or_else(|| values.get(key))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entries_with_comments_and_quotes() {
        let values = parse(
            "# comentario\nTELEDRIVE_API_ID=12345\nTELEDRIVE_API_HASH=\"abc123\"\nexport TURSO_TOKEN='xyz'\nsin_igual\n",
        );

        assert_eq!(
            values.get("TELEDRIVE_API_ID").map(String::as_str),
            Some("12345")
        );
        assert_eq!(
            values.get("TELEDRIVE_API_HASH").map(String::as_str),
            Some("abc123")
        );
        assert_eq!(values.get("TURSO_TOKEN").map(String::as_str), Some("xyz"));
        assert_eq!(get(&values, "API_ID").as_deref(), Some("12345"));
        assert_eq!(get(&values, "API_HASH").as_deref(), Some("abc123"));
        assert_eq!(get(&values, "TURSO_TOKEN").as_deref(), Some("xyz"));
        assert!(get(&values, "TURSO_URL").is_none());
    }
}
