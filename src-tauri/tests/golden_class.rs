//! Fichero dorado de la caja de clase, compartido con TypeScript.
//!
//! Igual que `golden_shapes`, pero con dos cosas en vez de una: el contorno
//! de los tres compartimentos y el texto con el que se escribe cada miembro.
//! El texto entra aqui porque tambien esta duplicado —`class_box.rs` y
//! `src/model/classBox.ts`— y un `+ nombre: Tipo` que en un sitio llevara
//! espacio y en el otro no se notaria solo al exportar.
//!
//! Para regenerarlo tras un cambio intencionado:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --manifest-path src-tauri/Cargo.toml golden
//! ```

use std::path::PathBuf;

use diagrammaker_lib::model::class_box::{ClassMember, Visibility};
use diagrammaker_lib::model::geometry::Rect;
use diagrammaker_lib::model::shape_geometry::{
    Primitive, class_attributes_top, class_height, class_operations_top, class_primitives,
    class_row_rect,
};
use serde::{Deserialize, Serialize};

/// Combinaciones a probar. Incluyen los casos degenerados a proposito: una
/// clase sin nada dentro es donde se nota el alto minimo de cada
/// compartimento.
const SHAPES: [(bool, usize, usize); 6] = [
    (false, 0, 0),
    (false, 1, 0),
    (false, 0, 1),
    (false, 3, 2),
    (true, 2, 2),
    (true, 0, 0),
];

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct BoxCase {
    stereotype: bool,
    attributes: usize,
    operations: usize,
    width: f64,
    height: f64,
    attributes_top: f64,
    operations_top: f64,
    /// Caja de la primera fila de cada compartimento.
    first_attribute: Rect,
    first_operation: Rect,
    primitives: Vec<Primitive>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct MemberCase {
    description: String,
    is_operation: bool,
    text: String,
}

fn box_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/class-primitives.json")
}

fn member_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/class-members.json")
}

fn boxes() -> Vec<BoxCase> {
    SHAPES
        .iter()
        .map(|&(stereotype, attributes, operations)| {
            let width = 200.0;
            let height = class_height(stereotype, attributes, operations);
            let top = class_attributes_top(stereotype);
            let bottom = class_operations_top(stereotype, attributes);

            BoxCase {
                stereotype,
                attributes,
                operations,
                width,
                height,
                attributes_top: top,
                operations_top: bottom,
                first_attribute: class_row_rect(top, 0, width),
                first_operation: class_row_rect(bottom, 0, width),
                primitives: class_primitives(stereotype, attributes, width, height),
            }
        })
        .collect()
}

fn members() -> Vec<MemberCase> {
    let cases: Vec<(&str, ClassMember, bool)> = vec![
        (
            "atributo con tipo",
            ClassMember::new("a", "nombre")
                .with_visibility(Visibility::Private)
                .with_type("String"),
            false,
        ),
        ("atributo sin tipo", ClassMember::new("a", "activo"), false),
        (
            "atributo protegido",
            ClassMember::new("a", "saldo")
                .with_visibility(Visibility::Protected)
                .with_type("Decimal"),
            false,
        ),
        (
            "atributo de paquete",
            ClassMember::new("a", "interno").with_visibility(Visibility::Package),
            false,
        ),
        ("operacion sin nada", ClassMember::new("o", "guardar"), true),
        (
            "operacion con parametros y retorno",
            ClassMember::new("o", "buscar")
                .with_parameters("id: int, activo: bool")
                .with_type("Pedido"),
            true,
        ),
        (
            "operacion que no devuelve nada",
            ClassMember::new("o", "borrar")
                .with_visibility(Visibility::Private)
                .with_parameters("id: int"),
            true,
        ),
    ];

    cases
        .into_iter()
        .map(|(description, member, is_operation)| MemberCase {
            description: description.to_string(),
            is_operation,
            text: member.text(is_operation),
        })
        .collect()
}

/// Compara contra el fichero, o lo escribe si se pidio regenerarlo.
fn check<T>(path: PathBuf, actual: Vec<T>, what: &str)
where
    T: Serialize + for<'de> Deserialize<'de> + PartialEq + std::fmt::Debug,
{
    let serialized = format!("{}\n", serde_json::to_string_pretty(&actual).unwrap());

    if std::env::var("UPDATE_GOLDEN").is_ok() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &serialized).unwrap();
        return;
    }

    let expected: Vec<T> = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    assert_eq!(
        actual,
        expected,
        "{what} ya no coincide con {}. Si el cambio es intencionado, \
         regenera con UPDATE_GOLDEN=1 y actualiza la copia de TypeScript.",
        path.display()
    );
}

#[test]
fn la_geometria_de_la_caja_de_clase_coincide_con_el_fichero_dorado() {
    check(box_path(), boxes(), "la geometria de la caja de clase");
}

#[test]
fn el_texto_de_los_miembros_coincide_con_el_fichero_dorado() {
    check(member_path(), members(), "el texto de los miembros");
}

#[test]
fn los_casos_de_miembro_tienen_descripciones_distintas() {
    // La descripcion es la clave con la que los busca la prueba de
    // TypeScript.
    let mut seen: Vec<String> = members().into_iter().map(|c| c.description).collect();
    let total = seen.len();
    seen.sort();
    seen.dedup();

    assert_eq!(seen.len(), total, "hay descripciones repetidas");
}
