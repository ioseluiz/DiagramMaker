//! Fichero dorado del reparto de una secuencia, compartido con TypeScript.
//!
//! `src/model/laneGeometry.ts` reimplementa `model::lane_geometry` para poder
//! dibujar el lienzo de carriles sin cruzar el puente IPC en cada repintado.
//! Esta prueba y su gemela en `src/model/laneGeometry.test.ts` leen el
//! **mismo** JSON, asi que si una de las dos implementaciones cambia sin la
//! otra, una de las dos falla.
//!
//! Para regenerarlo tras un cambio intencionado:
//!
//! ```sh
//! UPDATE_GOLDEN=1 cargo test --manifest-path src-tauri/Cargo.toml golden
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use diagrammaker_lib::model::lane_geometry::{LaneLayout, Measures, layout};
use diagrammaker_lib::model::{
    Block, BlockKind, Branch, Message, MessageKind, NotePlacement, SequenceNote, SequenceScript,
    Size, Step,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Case {
    name: String,
    participants: Vec<String>,
    script: SequenceScript,
    measures: Measures,
    layout: LaneLayout,
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/golden/lane-layout.json")
}

fn ids(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| n.to_string()).collect()
}

/// Medidas fijas, no medidas de verdad: lo que se vigila es la aritmetica que
/// se hace con ellas, no el ancho real de una tipografia.
fn measures(participants: &[(&str, f64)], texts: &[(&str, f64)]) -> Measures {
    let mut out = Measures::default();
    for (id, width) in participants {
        out.headers.insert(id.to_string(), Size::new(*width, 40.0));
    }
    for (id, width) in texts {
        out.texts.insert(id.to_string(), Size::new(*width, 14.0));
    }
    out
}

fn message(id: &str, from: &str, to: &str, kind: MessageKind) -> Message {
    Message::new(id, from, to, kind)
}

/// Los casos cubren cada pieza del reparto: lo que no este aqui puede cambiar
/// en una de las dos implementaciones sin que nadie se entere.
fn cases() -> Vec<Case> {
    // 5. (declarado antes por anidarse dentro de otro marco)
    let interior = Step::Block(Block {
        id: "dentro".into(),
        kind: BlockKind::Opt,
        branches: vec![
            Branch::new("r3", "si hay stock").with_steps(vec![Step::Message(message(
                "m2",
                "b",
                "a",
                MessageKind::Reply,
            ))]),
        ],
    });

    let mut cases = vec![
        // 1. Lo minimo: dos participantes y una llamada con respuesta.
        Case {
            name: "llamada y respuesta".into(),
            participants: ids(&["a", "b"]),
            script: SequenceScript {
                steps: vec![
                    Step::Message(message("m1", "a", "b", MessageKind::Sync).with_text("pedir")),
                    Step::Message(message("m2", "b", "a", MessageKind::Reply).with_text("dar")),
                ],
            },
            measures: measures(&[("a", 100.0), ("b", 140.0)], &[("m1", 44.0), ("m2", 30.0)]),
            layout: LaneLayout::default(),
        },
        // 2. Barras de ejecucion anidadas, que es donde esta el desplazamiento
        //    por profundidad y el recorte de las flechas.
        Case {
            name: "barras anidadas".into(),
            participants: ids(&["a", "b", "c"]),
            script: SequenceScript {
                steps: vec![
                    Step::Message(message("m1", "a", "b", MessageKind::Sync).activating()),
                    Step::Message(message("m2", "a", "b", MessageKind::Sync).activating()),
                    Step::Message(message("m3", "b", "c", MessageKind::Sync).activating()),
                    Step::Message(message("m4", "c", "b", MessageKind::Reply).deactivating()),
                    Step::Message(message("m5", "b", "a", MessageKind::Reply).deactivating()),
                ],
            },
            measures: measures(&[("a", 90.0), ("b", 90.0), ("c", 90.0)], &[]),
            layout: LaneLayout::default(),
        },
        // 3. Un mensaje a uno mismo: reserva sitio a su derecha y alto propio.
        Case {
            name: "mensaje a uno mismo".into(),
            participants: ids(&["a", "b"]),
            script: SequenceScript {
                steps: vec![Step::Message(
                    message("m1", "a", "a", MessageKind::Sync).with_text("validar"),
                )],
            },
            measures: measures(&[("a", 90.0), ("b", 90.0)], &[("m1", 120.0)]),
            layout: LaneLayout::default(),
        },
        // 4. Un texto largo que cruza dos huecos: el deficit se reparte.
        Case {
            name: "texto que ensancha".into(),
            participants: ids(&["a", "b", "c"]),
            script: SequenceScript {
                steps: vec![Step::Message(
                    message("m1", "a", "c", MessageKind::Async).with_text("muy largo"),
                )],
            },
            measures: measures(&[("a", 90.0), ("b", 90.0), ("c", 90.0)], &[("m1", 620.0)]),
            layout: LaneLayout::default(),
        },
        // 5. Marcos anidados con varias ramas.
        Case {
            name: "marcos anidados".into(),
            participants: ids(&["a", "b"]),
            script: SequenceScript {
                steps: vec![Step::Block(Block {
                    id: "fuera".into(),
                    kind: BlockKind::Alt,
                    branches: vec![
                        Branch::new("r1", "ok").with_steps(vec![
                            Step::Message(message("m1", "a", "b", MessageKind::Sync)),
                            interior,
                        ]),
                        Branch::new("r2", "error").with_steps(vec![Step::Message(message(
                            "m3",
                            "a",
                            "b",
                            MessageKind::Sync,
                        ))]),
                    ],
                })],
            },
            measures: measures(&[("a", 90.0), ("b", 90.0)], &[]),
            layout: LaneLayout::default(),
        },
        // 6. Crear, destruir y notas: lo que mueve cabeceras y lineas de vida.
        Case {
            name: "crear destruir y notas".into(),
            participants: ids(&["a", "b", "c"]),
            script: SequenceScript {
                steps: vec![
                    Step::Note(SequenceNote {
                        id: "n1".into(),
                        placement: NotePlacement::RightOf,
                        participants: ids(&["a"]),
                        text: "arranca".into(),
                    }),
                    Step::Message(message("m1", "a", "b", MessageKind::Create).with_text("nuevo")),
                    Step::Note(SequenceNote {
                        id: "n2".into(),
                        placement: NotePlacement::Over,
                        participants: ids(&["a", "c"]),
                        text: "las tres".into(),
                    }),
                    Step::Message(message("m2", "a", "b", MessageKind::Destroy)),
                ],
            },
            measures: measures(
                &[("a", 90.0), ("b", 120.0), ("c", 90.0)],
                &[("m1", 40.0), ("n1", 60.0), ("n2", 70.0)],
            ),
            layout: LaneLayout::default(),
        },
        // 7. Un guion vacio: el caso degenerado tambien tiene que coincidir.
        Case {
            name: "sin pasos".into(),
            participants: ids(&["solo"]),
            script: SequenceScript::default(),
            measures: measures(&[("solo", 200.0)], &[]),
            layout: LaneLayout::default(),
        },
    ];

    for case in &mut cases {
        case.layout = layout(&case.participants, &case.script, &case.measures);
    }

    cases
}

#[test]
fn el_reparto_de_secuencia_coincide_con_el_fichero_dorado() {
    let path = golden_path();
    let cases = cases();
    let serialized = format!("{}\n", serde_json::to_string_pretty(&cases).unwrap());

    if std::env::var("UPDATE_GOLDEN").is_ok() || !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, &serialized).unwrap();
        return;
    }

    let expected: Vec<Case> =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();

    assert_eq!(
        cases,
        expected,
        "el reparto ya no coincide con {}. Si el cambio es intencionado, \
         regenera el fichero con UPDATE_GOLDEN=1 y actualiza tambien la copia \
         de TypeScript.",
        path.display()
    );
}

#[test]
fn los_casos_del_fichero_dorado_tienen_nombres_distintos() {
    // El nombre es la clave con la que la prueba de TypeScript los busca.
    let mut seen = BTreeMap::new();
    for case in cases() {
        assert!(
            seen.insert(case.name.clone(), ()).is_none(),
            "el caso `{}` esta repetido",
            case.name
        );
    }
}
