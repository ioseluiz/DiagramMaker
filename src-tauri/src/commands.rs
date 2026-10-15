use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;

/// Version del esquema del documento `.dgm`. Se incrementa en cada cambio
/// incompatible del IR; `io::` usa este numero para migrar al abrir.
pub const DOCUMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
    pub schema_version: u32,
}

/// Identidad del backend. Sirve ademas como prueba de vida del puente IPC.
#[tauri::command]
pub fn app_info() -> AppResult<AppInfo> {
    Ok(AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        schema_version: DOCUMENT_SCHEMA_VERSION,
    })
}
