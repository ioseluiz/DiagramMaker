use crate::model::er::{ErColumn, ErTable};
use crate::model::node::Node;
use crate::model::sequence::{Block, BlockKind, Branch, Message, MessageKind, Step};
use crate::model::validate::validate;

use super::*;

fn doc(kind: DiagramKind) -> Document {
    Document::new("d1", kind, 0)
}

fn node(doc: &mut Document, id: &str, label: &str, shape: NodeShape) {
    doc.nodes.push(Node::new(id, label, shape));
}

fn edge(doc: &mut Document, id: &str, from: &str, to: &str, kind: EdgeKind) {
    doc.edges
        .push(crate::model::edge::Edge::new(id, from, to, kind));
}

/// Los codigos que salen, ordenados: la lista es lo que de verdad importa.
///
/// Se entra por `validate`, que es por donde entra la aplicacion, para que
/// una comprobacion que no estuviera enganchada no pasara desapercibida.
fn codes(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = validate(doc)
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect();
    out.sort();
    out
}

/// El nodo o arista al que apunta el primer diagnostico con ese codigo.
fn target(doc: &Document, code: &str) -> Option<String> {
    validate(doc)
        .diagnostics
        .into_iter()
        .find(|d| d.code == code)
        .and_then(|d| d.target)
}

/* --- Entidad-relacion -------------------------------------------------------- */

fn table(doc: &mut Document, id: &str, label: &str, columns: Vec<ErColumn>) {
    doc.nodes.push(Node::table(
        id,
        label,
        ErTable {
            columns,
            ..Default::default()
        },
    ));
}

#[test]
fn una_tabla_sin_columnas_se_avisa() {
    let mut d = doc(DiagramKind::Er);
    table(&mut d, "t1", "clientes", vec![]);

    assert_eq!(codes(&d), ["emptyTable"]);
    assert_eq!(target(&d, "emptyTable").as_deref(), Some("t1"));
}

#[test]
fn dos_columnas_con_el_mismo_nombre_son_un_error() {
    let mut d = doc(DiagramKind::Er);
    table(
        &mut d,
        "t1",
        "clientes",
        vec![
            ErColumn::new("c1", "id").with_type("int"),
            ErColumn::new("c2", "ID").with_type("int"),
        ],
    );

    assert_eq!(codes(&d), ["duplicateColumn"]);
    assert!(!validate(&d).ok);
}

#[test]
fn una_tabla_correcta_no_dice_nada() {
    let mut d = doc(DiagramKind::Er);
    table(
        &mut d,
        "t1",
        "clientes",
        vec![ErColumn::new("c1", "id").with_type("int")],
    );

    assert!(codes(&d).is_empty());
}

#[test]
fn una_relacion_que_no_toca_una_tabla_se_avisa() {
    let mut d = doc(DiagramKind::Er);
    table(
        &mut d,
        "t1",
        "clientes",
        vec![ErColumn::new("c1", "id").with_type("int")],
    );
    node(&mut d, "n1", "Nota suelta", NodeShape::Rectangle);
    edge(&mut d, "e1", "t1", "n1", EdgeKind::Association);

    assert_eq!(codes(&d), ["relationOutsideTable"]);
}

/* --- Clases -------------------------------------------------------------------- */

#[test]
fn una_herencia_circular_es_un_error() {
    let mut d = doc(DiagramKind::Class);
    for (id, label) in [("a", "A"), ("b", "B"), ("c", "C")] {
        node(&mut d, id, label, NodeShape::Class);
    }
    edge(&mut d, "e1", "a", "b", EdgeKind::Inheritance);
    edge(&mut d, "e2", "b", "c", EdgeKind::Inheritance);
    edge(&mut d, "e3", "c", "a", EdgeKind::Inheritance);

    assert_eq!(codes(&d), ["inheritanceCycle"]);

    let mensaje = validate(&d).diagnostics[0].message.clone();
    assert!(mensaje.contains('A') && mensaje.contains('B') && mensaje.contains('C'));
}

#[test]
fn una_clase_que_hereda_de_si_misma_tambien() {
    let mut d = doc(DiagramKind::Class);
    node(&mut d, "a", "A", NodeShape::Class);
    edge(&mut d, "e1", "a", "a", EdgeKind::Inheritance);

    assert_eq!(codes(&d), ["inheritanceCycle"]);
}

#[test]
fn el_mismo_ciclo_no_se_cuenta_una_vez_por_clase() {
    let mut d = doc(DiagramKind::Class);
    node(&mut d, "a", "A", NodeShape::Class);
    node(&mut d, "b", "B", NodeShape::Class);
    edge(&mut d, "e1", "a", "b", EdgeKind::Inheritance);
    edge(&mut d, "e2", "b", "a", EdgeKind::Inheritance);

    assert_eq!(codes(&d).len(), 1);
}

#[test]
fn una_jerarquia_normal_no_dice_nada() {
    let mut d = doc(DiagramKind::Class);
    node(&mut d, "a", "A", NodeShape::Class);
    node(&mut d, "b", "B", NodeShape::Class);
    node(&mut d, "c", "C", NodeShape::Class);
    edge(&mut d, "e1", "b", "a", EdgeKind::Inheritance);
    edge(&mut d, "e2", "c", "a", EdgeKind::Inheritance);

    assert!(codes(&d).is_empty());
}

/* --- Extremos de un flujo ------------------------------------------------------ */

#[test]
fn una_flecha_que_entra_al_arranque_es_un_error() {
    let mut d = doc(DiagramKind::State);
    node(&mut d, "i", "", NodeShape::InitialNode);
    node(&mut d, "a", "A", NodeShape::RoundedRect);
    edge(&mut d, "e1", "i", "a", EdgeKind::Directed);
    edge(&mut d, "e2", "a", "i", EdgeKind::Directed);

    assert!(codes(&d).contains(&"arrowIntoStart".to_string()));
}

#[test]
fn una_flecha_que_sale_del_final_es_un_error() {
    let mut d = doc(DiagramKind::Activity);
    node(&mut d, "i", "", NodeShape::InitialNode);
    node(&mut d, "f", "", NodeShape::FinalNode);
    node(&mut d, "a", "A", NodeShape::RoundedRect);
    edge(&mut d, "e1", "i", "f", EdgeKind::Directed);
    edge(&mut d, "e2", "f", "a", EdgeKind::Directed);

    assert!(codes(&d).contains(&"arrowOutOfEnd".to_string()));
}

/* --- Maquinas de estados --------------------------------------------------------- */

#[test]
fn dos_arranques_en_el_mismo_nivel_son_un_error() {
    let mut d = doc(DiagramKind::State);
    node(&mut d, "i1", "", NodeShape::InitialNode);
    node(&mut d, "i2", "", NodeShape::InitialNode);
    node(&mut d, "a", "A", NodeShape::RoundedRect);
    edge(&mut d, "e1", "i1", "a", EdgeKind::Directed);
    edge(&mut d, "e2", "i2", "a", EdgeKind::Directed);

    assert_eq!(codes(&d), ["manyStarts"]);
}

#[test]
fn cada_compuesto_tiene_derecho_a_su_arranque() {
    let mut d = doc(DiagramKind::State);
    node(&mut d, "i1", "", NodeShape::InitialNode);
    node(&mut d, "p", "Activo", NodeShape::Package);
    node(&mut d, "i2", "", NodeShape::InitialNode);
    node(&mut d, "a", "A", NodeShape::RoundedRect);
    d.node_mut("i2").unwrap().parent = Some("p".into());
    d.node_mut("a").unwrap().parent = Some("p".into());
    edge(&mut d, "e1", "i1", "a", EdgeKind::Directed);
    edge(&mut d, "e2", "i2", "a", EdgeKind::Directed);

    assert!(codes(&d).is_empty());
}

#[test]
fn un_estado_al_que_no_se_llega_se_avisa() {
    let mut d = doc(DiagramKind::State);
    node(&mut d, "i", "", NodeShape::InitialNode);
    node(&mut d, "a", "Nuevo", NodeShape::RoundedRect);
    node(&mut d, "b", "Huerfano", NodeShape::RoundedRect);
    edge(&mut d, "e1", "i", "a", EdgeKind::Directed);

    assert_eq!(codes(&d), ["unreachableState"]);
    assert_eq!(target(&d, "unreachableState").as_deref(), Some("b"));
}

#[test]
fn sin_arranque_no_se_habla_de_alcanzabilidad() {
    // Sin arranque todo seria inalcanzable y el aviso no diria nada.
    let mut d = doc(DiagramKind::State);
    node(&mut d, "a", "A", NodeShape::RoundedRect);
    node(&mut d, "b", "B", NodeShape::RoundedRect);

    assert!(codes(&d).is_empty());
}

/* --- Actividad --------------------------------------------------------------------- */

#[test]
fn un_diagrama_de_actividad_sin_arranque_se_avisa() {
    let mut d = doc(DiagramKind::Activity);
    node(&mut d, "a", "Hacer algo", NodeShape::RoundedRect);
    node(&mut d, "f", "", NodeShape::FinalNode);
    edge(&mut d, "e1", "a", "f", EdgeKind::Directed);

    assert_eq!(codes(&d), ["noStart"]);
}

#[test]
fn un_rombo_con_una_sola_salida_no_decide_nada() {
    let mut d = doc(DiagramKind::Activity);
    node(&mut d, "i", "", NodeShape::InitialNode);
    node(&mut d, "c", "sigue?", NodeShape::Diamond);
    node(&mut d, "f", "", NodeShape::FinalNode);
    edge(&mut d, "e1", "i", "c", EdgeKind::Directed);
    edge(&mut d, "e2", "c", "f", EdgeKind::Directed);

    assert_eq!(codes(&d), ["decisionWithoutBranches"]);
}

#[test]
fn un_paso_sin_salida_que_no_es_final_se_avisa() {
    let mut d = doc(DiagramKind::Activity);
    node(&mut d, "i", "", NodeShape::InitialNode);
    node(&mut d, "a", "Hacer algo", NodeShape::RoundedRect);
    edge(&mut d, "e1", "i", "a", EdgeKind::Directed);

    assert_eq!(codes(&d), ["danglingStep"]);
    assert_eq!(target(&d, "danglingStep").as_deref(), Some("a"));
}

#[test]
fn una_nota_o_un_contenedor_no_cuentan_como_paso_colgando() {
    let mut d = doc(DiagramKind::Activity);
    node(&mut d, "i", "", NodeShape::InitialNode);
    node(&mut d, "f", "", NodeShape::FinalNode);
    node(&mut d, "n", "Recordatorio", NodeShape::Note);
    edge(&mut d, "e1", "i", "f", EdgeKind::Directed);

    assert!(codes(&d).is_empty());
}

#[test]
fn un_flujo_correcto_no_dice_nada() {
    let mut d = doc(DiagramKind::Activity);
    node(&mut d, "i", "", NodeShape::InitialNode);
    node(&mut d, "a", "Hacer algo", NodeShape::RoundedRect);
    node(&mut d, "f", "", NodeShape::FinalNode);
    edge(&mut d, "e1", "i", "a", EdgeKind::Directed);
    edge(&mut d, "e2", "a", "f", EdgeKind::Directed);

    assert!(codes(&d).is_empty());
}

/* --- Casos de uso ------------------------------------------------------------------- */

#[test]
fn una_asociacion_entre_dos_casos_de_uso_se_avisa() {
    let mut d = doc(DiagramKind::UseCase);
    node(&mut d, "u1", "Comprar", NodeShape::Ellipse);
    node(&mut d, "u2", "Pagar", NodeShape::Ellipse);
    edge(&mut d, "e1", "u1", "u2", EdgeKind::Association);

    assert_eq!(codes(&d), ["associationBetweenUseCases"]);
}

#[test]
fn una_dependencia_entre_casos_de_uso_es_lo_correcto() {
    let mut d = doc(DiagramKind::UseCase);
    node(&mut d, "a", "Cliente", NodeShape::Actor);
    node(&mut d, "u1", "Comprar", NodeShape::Ellipse);
    node(&mut d, "u2", "Pagar", NodeShape::Ellipse);
    edge(&mut d, "e1", "a", "u1", EdgeKind::Association);
    edge(&mut d, "e2", "u1", "u2", EdgeKind::Dependency);

    assert!(codes(&d).is_empty());
}

#[test]
fn una_dependencia_desde_un_actor_se_avisa() {
    let mut d = doc(DiagramKind::UseCase);
    node(&mut d, "a", "Cliente", NodeShape::Actor);
    node(&mut d, "u1", "Comprar", NodeShape::Ellipse);
    edge(&mut d, "e1", "a", "u1", EdgeKind::Dependency);

    assert_eq!(codes(&d), ["dependencyWithActor"]);
}

#[test]
fn un_caso_de_uso_al_que_no_llega_nadie_se_avisa() {
    let mut d = doc(DiagramKind::UseCase);
    node(&mut d, "a", "Cliente", NodeShape::Actor);
    node(&mut d, "u1", "Comprar", NodeShape::Ellipse);
    node(&mut d, "u2", "Suelto", NodeShape::Ellipse);
    edge(&mut d, "e1", "a", "u1", EdgeKind::Association);

    assert_eq!(codes(&d), ["uselessUseCase"]);
    assert_eq!(target(&d, "uselessUseCase").as_deref(), Some("u2"));
}

/* --- Secuencia ------------------------------------------------------------------------ */

fn participant(doc: &mut Document, id: &str, label: &str) {
    doc.nodes.push(Node::new(id, label, NodeShape::Rectangle));
}

#[test]
fn un_mensaje_a_un_participante_que_no_existe_es_un_error() {
    let mut d = doc(DiagramKind::Sequence);
    participant(&mut d, "p1", "Cliente");

    d.script.as_mut().unwrap().steps = vec![Step::Message(
        Message::new("m1", "p1", "fantasma", MessageKind::Sync).with_text("hola"),
    )];

    assert_eq!(codes(&d), ["unknownParticipant"]);
    assert!(!validate(&d).ok);
}

#[test]
fn un_guion_correcto_no_dice_nada() {
    let mut d = doc(DiagramKind::Sequence);
    participant(&mut d, "p1", "Cliente");
    participant(&mut d, "p2", "Tienda");

    d.script.as_mut().unwrap().steps = vec![Step::Message(
        Message::new("m1", "p1", "p2", MessageKind::Sync).with_text("comprar"),
    )];

    assert!(codes(&d).is_empty());
}

#[test]
fn una_rama_vacia_se_avisa() {
    let mut d = doc(DiagramKind::Sequence);
    participant(&mut d, "p1", "Cliente");

    d.script.as_mut().unwrap().steps = vec![Step::Block(Block {
        id: "b1".into(),
        kind: BlockKind::Alt,
        branches: vec![Branch::new("r1", "si")],
    })];

    assert_eq!(codes(&d), ["emptyBranch"]);
}

#[test]
fn un_bloque_sin_ramas_se_avisa() {
    let mut d = doc(DiagramKind::Sequence);
    participant(&mut d, "p1", "Cliente");

    d.script.as_mut().unwrap().steps = vec![Step::Block(Block {
        id: "b1".into(),
        kind: BlockKind::Opt,
        branches: vec![],
    })];

    assert_eq!(codes(&d), ["emptyBlock"]);
}

#[test]
fn se_mira_tambien_dentro_de_los_bloques() {
    let mut d = doc(DiagramKind::Sequence);
    participant(&mut d, "p1", "Cliente");

    d.script.as_mut().unwrap().steps = vec![Step::Block(Block {
        id: "b1".into(),
        kind: BlockKind::Alt,
        branches: vec![Branch::new("r1", "si").with_steps(vec![Step::Message(
            Message::new("m1", "p1", "fantasma", MessageKind::Sync).with_text("hola"),
        )])],
    })];

    assert_eq!(codes(&d), ["unknownParticipant"]);
}

#[test]
fn una_activacion_que_se_cierra_sin_abrirse_se_avisa() {
    let mut d = doc(DiagramKind::Sequence);
    participant(&mut d, "p1", "Cliente");
    participant(&mut d, "p2", "Tienda");

    d.script.as_mut().unwrap().steps = vec![Step::Message(
        Message::new("m1", "p1", "p2", MessageKind::Reply)
            .with_text("respuesta")
            .deactivating(),
    )];

    assert_eq!(codes(&d), ["unbalancedActivation"]);
}

#[test]
fn una_activacion_bien_emparejada_no_dice_nada() {
    let mut d = doc(DiagramKind::Sequence);
    participant(&mut d, "p1", "Cliente");
    participant(&mut d, "p2", "Tienda");

    d.script.as_mut().unwrap().steps = vec![
        Step::Message(
            Message::new("m1", "p1", "p2", MessageKind::Sync)
                .with_text("comprar")
                .activating(),
        ),
        Step::Message(
            Message::new("m2", "p2", "p1", MessageKind::Reply)
                .with_text("listo")
                .deactivating(),
        ),
    ];

    assert!(codes(&d).is_empty());
}

/* --- Lo que no se toca ------------------------------------------------------------------ */

#[test]
fn un_diagrama_de_arquitectura_no_tiene_reglas_propias() {
    // Una caja suelta en una arquitectura es normal: se esta dibujando.
    let mut d = doc(DiagramKind::Architecture);
    node(&mut d, "a", "Portal", NodeShape::Rectangle);
    node(&mut d, "b", "API", NodeShape::Rectangle);

    assert!(codes(&d).is_empty());
}

#[test]
fn las_reglas_de_un_tipo_no_se_aplican_a_otro() {
    // El mismo grafo que en actividad daria `noStart` y `danglingStep`.
    let mut d = doc(DiagramKind::Architecture);
    node(&mut d, "c", "sigue?", NodeShape::Diamond);
    node(&mut d, "a", "Paso", NodeShape::RoundedRect);
    edge(&mut d, "e1", "c", "a", EdgeKind::Directed);

    assert!(codes(&d).is_empty());
}
