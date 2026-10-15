//! Estado del espacio de trabajo que sobrevive al cierre de la aplicacion:
//! archivos recientes y copia de recuperacion.
//!
//! Vive en el directorio de datos de la aplicacion, no junto a los diagramas:
//! es estado de la herramienta, no del documento, y no tiene por que acabar en
//! el repositorio de nadie.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::error::AppResult;
use crate::model::Document;

const RECENTS_FILE: &str = "recents.json";
const RECOVERY_FILE: &str = "recovery.dgm";
const RECOVERY_META: &str = "recovery.json";

/// Cuantos archivos recientes se recuerdan.
pub const RECENTS_LIMIT: usize = 12;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct RecentFile {
    pub path: String,
    pub name: String,
    /// `false` si el archivo ya no esta donde estaba.
    pub exists: bool,
}

/// Copia de recuperacion pendiente de un cierre inesperado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Recovery {
    pub document: Document,
    /// Ruta original del documento, si lo habian guardado alguna vez.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub path: Option<String>,
    /// Milisegundos desde el epoch Unix; ver la nota de `Meta` sobre por que
    /// se declara como `number` y no como `bigint`.
    #[ts(type = "number")]
    pub saved_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecoveryMeta {
    path: Option<String>,
    saved_at: i64,
}

fn file_name_of(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
}

/* --- Recientes --------------------------------------------------------- */

pub fn recents(dir: &Path) -> Vec<RecentFile> {
    let Ok(text) = fs::read_to_string(dir.join(RECENTS_FILE)) else {
        return Vec::new();
    };

    serde_json::from_str::<Vec<String>>(&text)
        .unwrap_or_default()
        .into_iter()
        .map(|path| RecentFile {
            name: file_name_of(&path),
            exists: Path::new(&path).is_file(),
            path,
        })
        .collect()
}

/// Mueve `path` al principio de la lista. Si ya estaba, no se duplica.
pub fn remember(dir: &Path, path: &str) -> AppResult<Vec<RecentFile>> {
    let mut paths: Vec<String> = recents(dir).into_iter().map(|r| r.path).collect();
    paths.retain(|p| p != path);
    paths.insert(0, path.to_string());
    paths.truncate(RECENTS_LIMIT);

    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(RECENTS_FILE),
        serde_json::to_string_pretty(&paths)?,
    )?;

    Ok(recents(dir))
}

pub fn forget(dir: &Path, path: &str) -> AppResult<Vec<RecentFile>> {
    let mut paths: Vec<String> = recents(dir).into_iter().map(|r| r.path).collect();
    paths.retain(|p| p != path);

    fs::create_dir_all(dir)?;
    fs::write(
        dir.join(RECENTS_FILE),
        serde_json::to_string_pretty(&paths)?,
    )?;

    Ok(recents(dir))
}

/* --- Recuperacion ------------------------------------------------------- */

/// Guarda la copia de recuperacion. No toca el archivo del usuario: si la
/// aplicacion muere, su `.dgm` sigue como la ultima vez que el pulso Guardar.
pub fn write_recovery(
    dir: &Path,
    doc: &Document,
    path: Option<&str>,
    now_ms: i64,
) -> AppResult<()> {
    fs::create_dir_all(dir)?;
    fs::write(dir.join(RECOVERY_FILE), super::to_json(doc)?)?;
    fs::write(
        dir.join(RECOVERY_META),
        serde_json::to_string_pretty(&RecoveryMeta {
            path: path.map(str::to_string),
            saved_at: now_ms,
        })?,
    )?;
    Ok(())
}

/// Lee la copia pendiente, si la hay. No la borra: eso lo decide el usuario al
/// responder si quiere recuperarla.
pub fn read_recovery(dir: &Path) -> Option<Recovery> {
    let document = super::from_json(&fs::read_to_string(dir.join(RECOVERY_FILE)).ok()?).ok()?;
    let meta: RecoveryMeta =
        serde_json::from_str(&fs::read_to_string(dir.join(RECOVERY_META)).ok()?).ok()?;

    Some(Recovery {
        document,
        path: meta.path,
        saved_at: meta.saved_at,
    })
}

pub fn clear_recovery(dir: &Path) -> AppResult<()> {
    for name in [RECOVERY_FILE, RECOVERY_META] {
        match fs::remove_file(dir.join(name)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramKind, Node, NodeShape};
    use std::path::PathBuf;

    /// Directorio temporal propio de cada prueba, para que no se pisen.
    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("diagrammaker-ws-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn doc() -> Document {
        let mut d = Document::new("d1", DiagramKind::Er, 1_700_000_000_000);
        d.nodes
            .push(Node::new("a", "Usuarios", NodeShape::Rectangle).at(0.0, 0.0, 160.0, 60.0));
        d
    }

    #[test]
    fn sin_archivo_la_lista_de_recientes_esta_vacia() {
        assert!(recents(&temp_dir("vacio")).is_empty());
    }

    #[test]
    fn remember_pone_el_ultimo_primero() {
        let dir = temp_dir("orden");

        remember(&dir, "C:/a.dgm").unwrap();
        let list = remember(&dir, "C:/b.dgm").unwrap();

        assert_eq!(
            list.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            ["C:/b.dgm", "C:/a.dgm"]
        );
    }

    #[test]
    fn recordar_de_nuevo_mueve_al_frente_sin_duplicar() {
        let dir = temp_dir("dup");

        remember(&dir, "C:/a.dgm").unwrap();
        remember(&dir, "C:/b.dgm").unwrap();
        let list = remember(&dir, "C:/a.dgm").unwrap();

        assert_eq!(list.len(), 2);
        assert_eq!(list[0].path, "C:/a.dgm");
    }

    #[test]
    fn la_lista_se_corta_en_el_limite() {
        let dir = temp_dir("limite");

        let mut list = Vec::new();
        for i in 0..RECENTS_LIMIT + 5 {
            list = remember(&dir, &format!("C:/{i}.dgm")).unwrap();
        }

        assert_eq!(list.len(), RECENTS_LIMIT);
        assert_eq!(list[0].path, format!("C:/{}.dgm", RECENTS_LIMIT + 4));
    }

    #[test]
    fn marca_como_inexistente_lo_que_ya_no_esta_en_disco() {
        let dir = temp_dir("existe");
        let real = dir.join("real.dgm");
        super::super::save(&real, &doc()).unwrap();

        remember(&dir, real.to_str().unwrap()).unwrap();
        let list = remember(&dir, "C:/no-existe.dgm").unwrap();

        assert!(!list[0].exists);
        assert!(list[1].exists);
    }

    #[test]
    fn forget_quita_una_entrada() {
        let dir = temp_dir("forget");
        remember(&dir, "C:/a.dgm").unwrap();
        remember(&dir, "C:/b.dgm").unwrap();

        let list = forget(&dir, "C:/a.dgm").unwrap();

        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "C:/b.dgm");
    }

    #[test]
    fn el_nombre_sale_de_la_ruta() {
        let dir = temp_dir("nombre");
        let list = remember(&dir, "C:/proyectos/esquema.dgm").unwrap();

        assert_eq!(list[0].name, "esquema.dgm");
    }

    #[test]
    fn sin_copia_pendiente_no_hay_recuperacion() {
        assert!(read_recovery(&temp_dir("sin-copia")).is_none());
    }

    #[test]
    fn la_copia_de_recuperacion_vuelve_con_su_ruta_original() {
        let dir = temp_dir("recuperar");
        write_recovery(&dir, &doc(), Some("C:/x.dgm"), 42).unwrap();

        let recovered = read_recovery(&dir).expect("deberia haber copia");

        assert_eq!(recovered.document, doc());
        assert_eq!(recovered.path.as_deref(), Some("C:/x.dgm"));
        assert_eq!(recovered.saved_at, 42);
    }

    #[test]
    fn un_documento_sin_guardar_se_recupera_sin_ruta() {
        let dir = temp_dir("sin-ruta");
        write_recovery(&dir, &doc(), None, 7).unwrap();

        assert_eq!(read_recovery(&dir).unwrap().path, None);
    }

    #[test]
    fn limpiar_borra_la_copia_y_es_idempotente() {
        let dir = temp_dir("limpiar");
        write_recovery(&dir, &doc(), None, 1).unwrap();

        clear_recovery(&dir).unwrap();
        assert!(read_recovery(&dir).is_none());

        // Volver a limpiar cuando ya no hay nada no debe fallar.
        clear_recovery(&dir).unwrap();
    }
}
