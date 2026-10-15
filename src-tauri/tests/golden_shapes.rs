//! Fichero dorado compartido con TypeScript.
//!
//! `src/model/shapeGeometry.ts` reimplementa `model::shape_geometry` para poder
//! dibujar en el lienzo sin cruzar el puente IPC en cada repintado. Esta prueba
//! y su gemela en `src/model/shapeGeometry.test.ts` leen el **mismo** JSON, asi
//! que si una de las dos implementaciones cambia sin la otra, una de las dos
//! falla.
//!
//! Para regenerarlo tras un cambio intencionado:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --manifest-path src-tauri/Cargo.toml golden
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use diagrammaker_lib::model::NodeShape;
use diagrammaker_lib::model::shape_geometry::{
    ALL_SHAPES, Primitive, primitives, table_height, table_primitives,
};
use serde::{Deserialize, Serialize};

/// Tamanos de prueba. Incluyen casos degenerados a proposito: una caja
/// diminuta es donde los limites (`min`) de cada forma se notan.
const SIZES: [(f64, f64); 4] = [(160.0, 60.0), (120.0, 90.0), (40.0, 80.0), (24.0, 20.0)];

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Case {
    shape: String,
    width: f64,
    height: f64,
    primitives: Vec<Primitive>,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/shape-primitives.json")
}

fn table_golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/table-primitives.json")
}

/// Numeros de columnas a probar: ninguna, una, varias y mas de las que caben.
const TABLE_ROWS: [usize; 4] = [0, 1, 4, 12];

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct TableCase {
    rows: usize,
    width: f64,
    height: f64,
    primitives: Vec<Primitive>,
}

fn current_tables() -> Vec<TableCase> {
    TABLE_ROWS
        .iter()
        .map(|&rows| {
            let width = 220.0;
            let height = table_height(rows);
            TableCase {
                rows,
                width,
                height,
                primitives: table_primitives(rows, width, height),
            }
        })
        .collect()
}

fn shape_name(shape: NodeShape) -> String {
    // El mismo nombre que usa serde, que es el que ve TypeScript.
    serde_json::to_value(shape)
        .unwrap()
        .as_str()
        .unwrap()
        .to_string()
}

fn current() -> Vec<Case> {
    let mut cases = Vec::new();
    // BTreeMap para que el orden del fichero sea estable entre ejecuciones.
    let by_name: BTreeMap<String, NodeShape> =
        ALL_SHAPES.iter().map(|&s| (shape_name(s), s)).collect();

    for (name, shape) in by_name {
        for (width, height) in SIZES {
            cases.push(Case {
                shape: name.clone(),
                width,
                height,
                primitives: primitives(shape, width, height),
            });
        }
    }
    cases
}

#[test]
fn las_primitivas_de_forma_coinciden_con_el_fichero_dorado() {
    let path = golden_path();
    let cases = current();
    let serialized = format!("{}\n", serde_json::to_string_pretty(&cases).unwrap());

    if std::env::var("UPDATE_GOLDEN").is_ok() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &serialized).unwrap();
        return;
    }

    let expected: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    assert_eq!(
        cases, expected,
        "las primitivas cambiaron. Si es intencionado, regenera con \
         UPDATE_GOLDEN=1 y revisa que src/model/shapeGeometry.ts siga igual."
    );
}

#[test]
fn las_primitivas_de_tabla_coinciden_con_el_fichero_dorado() {
    let path = table_golden_path();
    let cases = current_tables();
    let serialized = format!(
        "{}
",
        serde_json::to_string_pretty(&cases).unwrap()
    );

    if std::env::var("UPDATE_GOLDEN").is_ok() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &serialized).unwrap();
        return;
    }

    let expected: Vec<TableCase> =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    assert_eq!(
        cases, expected,
        "la geometria de las tablas cambio. Si es intencionado, regenera con          UPDATE_GOLDEN=1 y revisa que src/model/shapeGeometry.ts y          src/model/tableMetrics.ts sigan igual."
    );
}
