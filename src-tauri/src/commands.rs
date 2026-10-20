use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use tauri::Manager;

use crate::emit;
use crate::error::AppResult;
use crate::io;
use crate::io::workspace::{self, RecentFile, Recovery};
use crate::model::{DiagramKind, Document, Report, SCHEMA_VERSION, validate};
use crate::parse::{self, ImportResult, SourceLanguage};
use crate::render::{self, RenderInput};
use crate::templates::{self, Template};

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

/// Diagramas de arranque. Son codigo: el frontend los importa como
/// cualquier otro texto.
#[tauri::command]
pub fn template_list() -> AppResult<Vec<Template>> {
    Ok(templates::all())
}

/* --- Importacion y generacion de codigo --------------------------------- */

/// Convierte codigo en un documento.
///
/// El documento sale **sin geometria**: colocar los nodos es cosa del
/// auto-layout del frontend, que es quien puede medir el texto. Los
/// diagnosticos vienen con linea y columna para poder senalarlos en el editor.
#[tauri::command]
pub fn import_code(
    language: SourceLanguage,
    source: String,
    now_ms: i64,
) -> AppResult<ImportResult> {
    Ok(parse::import(language, &source, now_ms))
}

/// Genera el codigo equivalente al documento.
#[tauri::command]
pub fn emit_code(language: SourceLanguage, document: Document) -> AppResult<String> {
    Ok(emit::emit(language, &document))
}

/// Escribe el codigo generado en un archivo.
#[tauri::command]
pub fn emit_code_to_file(
    language: SourceLanguage,
    document: Document,
    path: PathBuf,
) -> AppResult<()> {
    std::fs::write(&path, emit::emit(language, &document))?;
    Ok(())
}

/* --- Exportacion -------------------------------------------------------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum ExportFormat {
    Svg,
    Png,
    Pdf,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Svg => "svg",
            ExportFormat::Png => "png",
            ExportFormat::Pdf => "pdf",
        }
    }
}

/// SVG como texto, para copiarlo al portapapeles o previsualizarlo.
#[tauri::command]
pub fn export_svg(input: RenderInput) -> AppResult<String> {
    Ok(render::svg::render(&input))
}

/// Exporta a disco.
///
/// Los bytes no cruzan el puente IPC: un PNG a escala 4 de un diagrama grande
/// son decenas de megabytes, y serializarlos a JSON para que el frontend los
/// devuelva a Rust no tendria ningun sentido.
#[tauri::command]
pub fn export_to_file(input: RenderInput, path: PathBuf, format: ExportFormat) -> AppResult<()> {
    let svg = render::svg::render(&input);

    let bytes = match format {
        ExportFormat::Svg => svg.into_bytes(),
        ExportFormat::Png => render::raster::to_png(&svg, input.options.scale)?,
        ExportFormat::Pdf => render::pdf::to_pdf(&svg)?,
    };

    std::fs::write(&path, bytes)?;
    Ok(())
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
    fn import_code_devuelve_documento_y_diagnosticos() {
        let result = import_code(
            SourceLanguage::Dbml,
            "Table users {
  id integer [pk]
}"
            .into(),
            0,
        )
        .unwrap();

        assert!(result.ok());
        assert_eq!(result.document.nodes.len(), 1);
        assert_eq!(result.document.kind, DiagramKind::Er);
    }

    #[test]
    fn import_code_no_falla_ante_codigo_roto_sino_que_lo_reporta() {
        let result = import_code(SourceLanguage::Dbml, "esto no es dbml".into(), 0).unwrap();

        assert!(!result.ok());
        assert!(!result.diagnostics.is_empty());
    }

    #[test]
    fn emit_code_da_la_vuelta_a_import_code() {
        let imported = import_code(
            SourceLanguage::Dbml,
            "Table users {
  id integer [pk]
}"
            .into(),
            0,
        )
        .unwrap();

        let code = emit_code(SourceLanguage::Dbml, imported.document).unwrap();

        assert!(code.contains("Table users"));
        assert!(code.contains("[pk]"));
    }

    #[test]
    fn emit_code_to_file_escribe_lo_mismo_que_emit_code() {
        let imported = import_code(
            SourceLanguage::Dbml,
            "Table users {
  id integer [pk]
}"
            .into(),
            0,
        )
        .unwrap();

        let dir = std::env::temp_dir().join("diagrammaker-test-emit");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("esquema.dbml");

        emit_code_to_file(
            SourceLanguage::Dbml,
            imported.document.clone(),
            path.clone(),
        )
        .unwrap();

        let escrito = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            escrito,
            emit_code(SourceLanguage::Dbml, imported.document).unwrap()
        );

        std::fs::remove_file(&path).ok();
    }

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
