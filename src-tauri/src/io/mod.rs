//! Lectura y escritura de archivos `.dgm`.
//!
//! El formato es JSON indentado a proposito: un diagrama es algo que se
//! versiona junto al codigo que documenta, y un diff legible vale mas que unos
//! bytes de menos.

pub mod workspace;

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::error::{AppError, AppResult};
use crate::model::{Document, SCHEMA_VERSION};

pub const FILE_EXTENSION: &str = "dgm";

/// Lee un documento, migrandolo si viene de una version anterior del esquema.
pub fn load(path: &Path) -> AppResult<Document> {
    let text = fs::read_to_string(path)?;
    from_json(&text)
}

/// Escribe el documento. Primero vuelca a un archivo temporal y despues lo
/// renombra, para que una interrupcion a mitad de escritura no deje el `.dgm`
/// original truncado.
pub fn save(path: &Path, doc: &Document) -> AppResult<()> {
    let json = to_json(doc)?;

    let temp = path.with_extension(format!("{FILE_EXTENSION}.tmp"));
    fs::write(&temp, json)?;
    fs::rename(&temp, path)?;

    Ok(())
}

pub fn to_json(doc: &Document) -> AppResult<String> {
    serde_json::to_string_pretty(doc)
        .map_err(|e| AppError::InvalidDocument(format!("no se pudo serializar: {e}")))
}

pub fn from_json(text: &str) -> AppResult<Document> {
    let raw: Value = serde_json::from_str(text)
        .map_err(|e| AppError::InvalidDocument(format!("JSON mal formado: {e}")))?;

    let version = raw
        .get("schemaVersion")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            AppError::InvalidDocument(
                "falta `schemaVersion`: el archivo no parece un documento de DiagramMaker."
                    .to_string(),
            )
        })? as u32;

    let migrated = migrate(raw, version)?;

    serde_json::from_value(migrated)
        .map_err(|e| AppError::InvalidDocument(format!("estructura invalida: {e}")))
}

/// Punto unico de migracion entre versiones del esquema.
///
/// Hoy solo hay la version 1, asi que el unico trabajo real es rechazar
/// archivos de versiones futuras con un mensaje util en vez de un error de
/// deserializacion ilegible.
fn migrate(doc: Value, from: u32) -> AppResult<Value> {
    match from {
        v if v == SCHEMA_VERSION => Ok(doc),
        v if v > SCHEMA_VERSION => Err(AppError::Unsupported(format!(
            "el archivo usa el esquema v{v} y esta version de DiagramMaker entiende hasta v{SCHEMA_VERSION}. Actualiza la aplicacion."
        ))),
        v => Err(AppError::Unsupported(format!(
            "no hay migracion definida desde el esquema v{v}."
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramKind, Edge, EdgeKind, Node, NodeShape};

    fn sample() -> Document {
        let mut d = Document::new("d1", DiagramKind::Architecture, 1_700_000_000_000);
        d.nodes
            .push(Node::new("a", "API", NodeShape::Rectangle).at(0.0, 0.0, 160.0, 60.0));
        d.nodes
            .push(Node::new("b", "Base de datos", NodeShape::Cylinder).at(300.0, 0.0, 120.0, 90.0));
        d.edges
            .push(Edge::new("e1", "a", "b", EdgeKind::Flow).with_label("consulta"));
        d
    }

    #[test]
    fn un_documento_sobrevive_la_ida_y_vuelta_a_json() {
        let original = sample();
        let back = from_json(&to_json(&original).unwrap()).unwrap();

        assert_eq!(back, original);
    }

    #[test]
    fn el_json_usa_camel_case() {
        let json = to_json(&sample()).unwrap();

        assert!(json.contains("\"schemaVersion\""), "json: {json}");
        assert!(!json.contains("schema_version"), "json: {json}");
    }

    #[test]
    fn rechaza_un_archivo_sin_version_de_esquema() {
        let err = from_json(r#"{"id":"x","kind":"er"}"#).unwrap_err();

        assert!(matches!(err, AppError::InvalidDocument(_)));
        assert!(err.to_string().contains("schemaVersion"));
    }

    #[test]
    fn rechaza_un_esquema_mas_nuevo_pidiendo_actualizar() {
        let err = from_json(r#"{"schemaVersion":999}"#).unwrap_err();

        assert!(matches!(err, AppError::Unsupported(_)));
        assert!(err.to_string().contains("Actualiza"));
    }

    #[test]
    fn rechaza_json_mal_formado() {
        assert!(matches!(
            from_json("{ esto no es json").unwrap_err(),
            AppError::InvalidDocument(_)
        ));
    }

    #[test]
    fn save_y_load_recorren_el_disco() {
        let dir = std::env::temp_dir().join("diagrammaker-test-io");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prueba.dgm");

        let original = sample();
        save(&path, &original).unwrap();
        let back = load(&path).unwrap();

        assert_eq!(back, original);
        assert!(
            !path.with_extension("dgm.tmp").exists(),
            "el temporal debe desaparecer tras el renombrado"
        );

        fs::remove_file(&path).ok();
    }
}
