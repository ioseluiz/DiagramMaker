use super::*;
use crate::model::DiagramKind;
use crate::parse::mermaid::state_diagram as parser;

/// Importa, genera y vuelve a importar.
fn round_trip(source: &str) -> (Document, Document) {
    let first = parser::import(source, 0);
    assert!(
        first.ok(),
        "la primera importacion fallo: {:#?}",
        first.diagnostics
    );

    let code = emit(&first.document);
    let second = parser::import(&code, 0);
    assert!(
        second.ok(),
        "el codigo generado no vuelve a entrar:\n{code}\n{:#?}",
        second.diagnostics
    );

    (first.document, second.document)
}

/// Resumen de un documento, sin ids: la forma de cada nodo, de quien cuelga y
/// las transiciones por nombre.
fn shape_of(doc: &Document) -> Vec<String> {
    let name = |id: &str| -> String {
        match doc.node(id) {
            Some(n) if n.label.is_empty() => format!("{:?}", n.shape),
            Some(n) => n.label.clone(),
            None => "?".into(),
        }
    };

    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .map(|node| {
            format!(
                "{} {:?} en {}",
                if node.label.is_empty() {
                    format!("{:?}", node.shape)
                } else {
                    node.label.clone()
                },
                node.shape,
                node.parent.as_deref().map(name).unwrap_or("raiz".into()),
            )
        })
        .collect();
    out.sort();

    let mut edges: Vec<String> = doc
        .edges
        .iter()
        .map(|edge| {
            format!(
                "{} -> {} : {}",
                name(&edge.source.node),
                name(&edge.target.node),
                edge.label.clone().unwrap_or_default()
            )
        })
        .collect();
    edges.sort();
    out.extend(edges);

    out
}

fn assert_equivalent(a: &Document, b: &Document) {
    assert_eq!(a.direction, b.direction, "distinta direccion");
    assert_eq!(shape_of(a), shape_of(b), "distinto contenido");
}

#[test]
fn una_maquina_simple_sobrevive_la_ida_y_vuelta() {
    let (a, b) = round_trip("stateDiagram-v2\n  [*] --> Nuevo\n  Nuevo --> [*]");

    assert_equivalent(&a, &b);
}

#[test]
fn el_codigo_generado_es_legible() {
    let imported = parser::import("stateDiagram-v2\n  [*] --> Nuevo\n  Nuevo --> [*]", 0);

    assert_eq!(
        emit(&imported.document),
        "stateDiagram-v2\n\
         \x20 direction LR\n\
         \x20 [*] --> Nuevo\n\
         \x20 Nuevo --> [*]\n"
    );
}

#[test]
fn los_extremos_no_se_declaran() {
    // Un `[*]` es un sitio, no un estado: declararlo no significaria nada.
    let imported = parser::import("stateDiagram-v2\n  [*] --> A\n  A --> [*]", 0);
    let code = emit(&imported.document);

    assert!(!code.contains("state [*]"), "{code}");
    assert_eq!(code.matches("[*]").count(), 2, "{code}");
}

#[test]
fn un_estado_mencionado_por_una_transicion_no_necesita_declaracion() {
    let imported = parser::import("stateDiagram-v2\n  A --> B", 0);
    let code = emit(&imported.document);

    assert!(!code.contains("state A"), "sobra la declaracion:\n{code}");
    assert!(code.contains("A --> B"), "{code}");
}

#[test]
fn un_estado_suelto_si_la_necesita() {
    // Sin su linea desapareceria: ninguna transicion lo menciona.
    let (a, b) = round_trip("stateDiagram-v2\n  state Aislado");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes.len(), 1);
}

#[test]
fn conserva_una_descripcion_con_espacios() {
    let (a, b) = round_trip("stateDiagram-v2\n  state \"Pendiente de pago\" as p\n  p --> [*]");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes[0].label, "Pendiente de pago");
}

#[test]
fn conserva_las_barras_y_los_rombos() {
    let (a, b) = round_trip(
        "stateDiagram-v2\n\
         \x20 state f <<fork>>\n\
         \x20 state c <<choice>>\n\
         \x20 A --> f\n\
         \x20 A --> c",
    );

    assert_equivalent(&a, &b);
    assert_eq!(
        b.nodes.iter().filter(|n| n.shape == NodeShape::Bar).count(),
        1
    );
    assert_eq!(
        b.nodes
            .iter()
            .filter(|n| n.shape == NodeShape::Diamond)
            .count(),
        1
    );
}

#[test]
fn una_union_vuelve_escrita_como_bifurcacion() {
    // En el documento las dos son la misma forma, asi que elegir una al
    // escribir no cambia lo que se dibuja al releerlo.
    let (a, b) = round_trip("stateDiagram-v2\n  state j <<join>>\n  A --> j");

    assert_equivalent(&a, &b);
    assert!(emit(&a).contains("<<fork>>"));
}

#[test]
fn conserva_los_compuestos() {
    let (a, b) = round_trip(
        "stateDiagram-v2\n\
         \x20 state Activo {\n\
         \x20   Nuevo --> Pagado\n\
         \x20 }\n\
         \x20 [*] --> Activo",
    );

    assert_equivalent(&a, &b);
    let compuesto = b
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el compuesto");
    assert_eq!(b.children_of(Some(&compuesto.id)).count(), 2);
}

#[test]
fn una_transicion_a_un_extremo_se_escribe_dentro_de_su_compuesto() {
    // Es lo unico que de verdad depende del nivel: un `[*]` escrito en la
    // raiz seria el arranque del diagrama entero, no el del compuesto.
    let (a, b) = round_trip(
        "stateDiagram-v2\n\
         \x20 [*] --> Fuera\n\
         \x20 state Fuera {\n\
         \x20   [*] --> Dentro\n\
         \x20   Dentro --> [*]\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);

    // Dos discos y un anillo: uno de cada en la raiz no, porque la raiz solo
    // tiene arranque.
    let discos = b
        .nodes
        .iter()
        .filter(|n| n.shape == NodeShape::InitialNode)
        .count();
    assert_eq!(discos, 2, "uno por nivel");
}

#[test]
fn conserva_los_compuestos_anidados() {
    let (a, b) = round_trip(
        "stateDiagram-v2\n\
         \x20 state Fuera {\n\
         \x20   state Dentro {\n\
         \x20     A --> B\n\
         \x20   }\n\
         \x20 }",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_las_etiquetas_de_las_transiciones() {
    let (a, b) = round_trip("stateDiagram-v2\n  Nuevo --> Pagado : confirmar");

    assert_equivalent(&a, &b);
    assert_eq!(b.edges[0].label.as_deref(), Some("confirmar"));
}

#[test]
fn conserva_los_saltos_de_linea_de_una_etiqueta() {
    let (_, b) = round_trip("stateDiagram-v2\n  A --> B : primera<br/>segunda");

    assert_eq!(b.edges[0].label.as_deref(), Some("primera\nsegunda"));
}

#[test]
fn conserva_la_direccion() {
    for code in ["LR", "RL", "TB", "BT"] {
        let (a, b) = round_trip(&format!("stateDiagram-v2\n  direction {code}\n  A --> B"));
        assert_equivalent(&a, &b);
    }
}

#[test]
fn un_documento_vacio_genera_solo_la_cabecera() {
    let doc = Document::new("d", DiagramKind::State, 0);

    assert_eq!(emit(&doc), "stateDiagram-v2\n  direction LR\n");
}

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    let source = "stateDiagram-v2\n\
         \x20 direction LR\n\
         \n\
         \x20 [*] --> Nuevo\n\
         \x20 Nuevo --> Pagado : confirmar\n\
         \n\
         \x20 state Pagado {\n\
         \x20   [*] --> Cobrando\n\
         \x20   Cobrando --> Cobrado\n\
         \x20   Cobrado --> [*]\n\
         \x20 }\n\
         \n\
         \x20 state reparto <<fork>>\n\
         \x20 Pagado --> reparto\n\
         \x20 reparto --> Enviado\n\
         \x20 reparto --> Facturado\n\
         \n\
         \x20 state cierre <<join>>\n\
         \x20 Enviado --> cierre\n\
         \x20 Facturado --> cierre\n\
         \x20 cierre --> [*]";

    let (a, b) = round_trip(source);
    assert_equivalent(&a, &b);
}
