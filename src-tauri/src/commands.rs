use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use tauri::Manager;

use crate::error::AppResult;
use crate::io;
use crate::io::workspace::{self, RecentFile, Recovery};
use crate::model::{DiagramKind, Document, Report, SCHEMA_VERSION, validate};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub schema_version: u32,
    pub file_extension: String,
}

/// Identidad del backend. Sirve ademas como prueba de vida del puente IPC.
#[tauri::command]
pub fn app_info() -> AppResult<AppInfo> {
    Ok(AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version: SCHEMA_VERSION,
        file_extension: io::FILE_EXTENSION.to_string(),
    })
}

/// Documento vacio del tipo pedido. `now_ms` llega del frontend porque es quien
/// conoce el reloj y la zona horaria del usuario.
#[tauri::command]
pub fn document_new(kind: DiagramKind, now_ms: i64) -> AppResult<Document> {
    Ok(Document::new(
        uuid::Uuid::new_v4().to_string(),
        kind,
        now_ms,
    ))
}

#[tauri::command]
pub fn document_open(path: PathBuf) -> AppResult<Document> {
    io::load(&path)
}

#[tauri::command]
pub fn document_save(path: PathBuf, document: Document) -> AppResult<()> {
    io::save(&path, &document)
}

/// Valida las invariantes estructurales. El frontend la llama antes de guardar
/// y antes de exportar; nunca bloquea la edicion, solo informa.
#[tauri::command]
pub fn document_validate(document: Document) -> AppResult<Report> {
    Ok(validate(&document))
}

/* --- Espacio de trabajo: recientes y recuperacion ----------------------- */

/// Directorio de datos de la aplicacion, creandolo si hace falta.
fn workspace_dir(app: &tauri::AppHandle) -> AppResult<std::path::PathBuf> {
    let dir = app.path().app_data_dir().map_err(|e| {
        crate::error::AppError::Unsupported(format!(
            "no se pudo resolver el directorio de datos de la aplicacion: {e}"
        ))
    })?;
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[tauri::command]
pub fn recent_list(app: tauri::AppHandle) -> AppResult<Vec<RecentFile>> {
    Ok(workspace::recents(&workspace_dir(&app)?))
}

#[tauri::command]
pub fn recent_remember(app: tauri::AppHandle, path: String) -> AppResult<Vec<RecentFile>> {
    workspace::remember(&workspace_dir(&app)?, &path)
}

#[tauri::command]
pub fn recent_forget(app: tauri::AppHandle, path: String) -> AppResult<Vec<RecentFile>> {
    workspace::forget(&workspace_dir(&app)?, &path)
}

/// Escribe la copia de recuperacion. Nunca toca el archivo del usuario.
#[tauri::command]
pub fn autosave_write(
    app: tauri::AppHandle,
    document: Document,
    path: Option<String>,
    now_ms: i64,
) -> AppResult<()> {
    workspace::write_recovery(&workspace_dir(&app)?, &document, path.as_deref(), now_ms)
}

#[tauri::command]
pub fn autosave_read(app: tauri::AppHandle) -> AppResult<Option<Recovery>> {
    Ok(workspace::read_recovery(&workspace_dir(&app)?))
}

#[tauri::command]
pub fn autosave_clear(app: tauri::AppHandle) -> AppResult<()> {
    workspace::clear_recovery(&workspace_dir(&app)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_info_reporta_la_version_del_esquema_y_la_extension() {
        let info = app_info().unwrap();

        assert_eq!(info.schema_version, SCHEMA_VERSION);
        assert_eq!(info.file_extension, "dgm");
        assert_eq!(info.name, "diagrammaker");
    }

    #[test]
    fn document_new_crea_un_documento_vacio_y_valido() {
        let d = document_new(DiagramKind::Er, 1_700_000_000_000).unwrap();

        assert_eq!(d.kind, DiagramKind::Er);
        assert_eq!(d.meta.title, "Entidad-relacion");
        assert_eq!(d.meta.created, d.meta.modified);
        assert!(d.nodes.is_empty());
        assert!(validate(&d).ok);
    }

    #[test]
    fn document_new_da_un_id_distinto_cada_vez() {
        let a = document_new(DiagramKind::Er, 0).unwrap();
        let b = document_new(DiagramKind::Er, 0).unwrap();

        assert_ne!(a.id, b.id);
    }

    #[test]
    fn document_validate_devuelve_el_informe_sin_fallar() {
        let mut d = document_new(DiagramKind::Architecture, 0).unwrap();
        d.edges.push(crate::model::Edge::new(
            "e1",
            "fantasma",
            "otro",
            crate::model::EdgeKind::Flow,
        ));

        let report = document_validate(d).unwrap();

        assert!(!report.ok);
        assert_eq!(report.errors().count(), 2);
    }
}
