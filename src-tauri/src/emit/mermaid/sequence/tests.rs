use super::*;
use crate::model::{DiagramKind, SequenceScript};
use crate::parse::mermaid::sequence as parser;

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

fn script(doc: &Document) -> SequenceScript {
    doc.script.clone().unwrap_or_default()
}

fn assert_equivalent(a: &Document, b: &Document) {
    assert_eq!(
        a.nodes
            .iter()
            .map(|n| (&n.label, n.shape))
            .collect::<Vec<_>>(),
        b.nodes
            .iter()
            .map(|n| (&n.label, n.shape))
            .collect::<Vec<_>>(),
        "distintos participantes"
    );

    let (left, right) = (script(a), script(b));
    assert_eq!(
        shape_of(&left.steps),
        shape_of(&right.steps),
        "distinta forma del guion"
    );

    for (x, y) in left.messages().iter().zip(right.messages()) {
        assert_eq!(x.kind, y.kind, "tipo de `{}`", x.text);
        assert_eq!(x.text, y.text);
        assert_eq!(x.activate, y.activate, "activacion de `{}`", x.text);
        assert_eq!(x.deactivate, y.deactivate, "cierre de `{}`", x.text);
    }
}

/// Resumen de la estructura del guion, sin ids: es lo que tiene que coincidir.
fn shape_of(steps: &[Step]) -> Vec<String> {
    steps
        .iter()
        .map(|step| match step {
            Step::Message(m) => format!("msg {:?} {}", m.kind, m.text),
            Step::Note(n) => format!("note {:?} x{}", n.placement, n.participants.len()),
            Step::Block(b) => format!(
                "block {:?} [{}]",
                b.kind,
                b.branches
                    .iter()
                    .map(|branch| format!("{}:{}", branch.label, shape_of(&branch.steps).join("|")))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ),
        })
        .collect()
}

#[test]
fn una_llamada_y_su_retorno_sobreviven_la_ida_y_vuelta() {
    let (a, b) = round_trip("sequenceDiagram\n  A->>B: pedir\n  B-->>A: dar");

    assert_equivalent(&a, &b);
}

#[test]
fn el_codigo_generado_es_legible() {
    let imported = parser::import("sequenceDiagram\n  A->>B: pedir", 0);

    assert_eq!(
        emit(&imported.document),
        "sequenceDiagram\n\
         \x20 participant n1 as A\n\
         \x20 participant n2 as B\n\
         \n\
         \x20 n1->>n2: pedir\n"
    );
}

#[test]
fn conserva_los_tipos_de_mensaje() {
    let (a, b) = round_trip(
        "sequenceDiagram\n\
         \x20 A->>B: llamada\n\
         \x20 B-->>A: retorno\n\
         \x20 A-)B: asincrono",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_los_actores() {
    let (a, b) =
        round_trip("sequenceDiagram\n  actor Cliente\n  participant API\n  Cliente->>API: x");

    assert_equivalent(&a, &b);
    assert_eq!(b.nodes[0].shape, NodeShape::Actor);
}

#[test]
fn conserva_el_orden_de_los_participantes() {
    let (a, b) = round_trip("sequenceDiagram\n  participant Z\n  participant A\n  A->>Z: x");

    assert_equivalent(&a, &b);
    assert_eq!(
        b.nodes.iter().map(|n| n.label.as_str()).collect::<Vec<_>>(),
        ["Z", "A"]
    );
}

#[test]
fn conserva_las_barras_de_ejecucion() {
    let (a, b) = round_trip("sequenceDiagram\n  A->>+B: llamar\n  B-->>-A: volver");

    assert_equivalent(&a, &b);
    assert!(script(&b).messages()[0].activate);
    assert!(script(&b).messages()[1].deactivate);
}

#[test]
fn un_mensaje_que_abre_y_cierra_a_la_vez_escribe_las_dos_cosas() {
    // Mermaid solo admite un signo pegado al destino, asi que la segunda
    // mitad sale en su propia linea.
    let (a, b) = round_trip("sequenceDiagram\n  A->>+B: llamar\n  B->>+C: seguir\n  deactivate B");

    assert_equivalent(&a, &b);
    let segundo = &script(&b).messages()[1].clone();
    assert!(segundo.activate);
    assert!(segundo.deactivate);
}

#[test]
fn conserva_los_marcos_y_sus_ramas() {
    let (a, b) = round_trip(
        "sequenceDiagram\n\
         \x20 alt hay stock\n\
         \x20   A->>B: reservar\n\
         \x20 else no hay\n\
         \x20   A->>B: avisar\n\
         \x20 end",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_todos_los_tipos_de_marco() {
    for source in [
        "opt quiza",
        "loop cada hora",
        "par a la vez",
        "critical importante",
        "break si falla",
        "rect rgb(1,2,3)",
    ] {
        let (a, b) = round_trip(&format!("sequenceDiagram\n  {source}\n    A->>B: x\n  end"));
        assert_equivalent(&a, &b);
    }
}

#[test]
fn conserva_los_marcos_anidados() {
    let (a, b) = round_trip(
        "sequenceDiagram\n\
         \x20 alt fuera\n\
         \x20   opt dentro\n\
         \x20     A->>B: hondo\n\
         \x20   end\n\
         \x20 end",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_las_notas() {
    let (a, b) = round_trip(
        "sequenceDiagram\n\
         \x20 participant A\n\
         \x20 participant B\n\
         \x20 Note right of A: una\n\
         \x20 Note over A,B: las dos",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_la_creacion_y_la_destruccion() {
    let (a, b) = round_trip(
        "sequenceDiagram\n\
         \x20 participant A\n\
         \x20 A->>A: arrancar\n\
         \x20 create participant B\n\
         \x20 A->>B: nacer\n\
         \x20 destroy B\n\
         \x20 A->>B: morir",
    );

    assert_equivalent(&a, &b);
    let kinds: Vec<_> = script(&b).messages().iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        [MessageKind::Sync, MessageKind::Create, MessageKind::Destroy]
    );
}

#[test]
fn un_participante_creado_no_se_declara_arriba() {
    // Si se declarara, Mermaid lo dibujaria desde el principio y el `create`
    // no tendria sentido.
    let imported = parser::import(
        "sequenceDiagram\n  participant A\n  create participant B\n  A->>B: nacer",
        0,
    );
    let code = emit(&imported.document);

    assert_eq!(code.matches("participant n2").count(), 1, "{code}");
    assert!(code.contains("create participant n2"), "{code}");
}

#[test]
fn conserva_el_titulo() {
    let (a, b) = round_trip("sequenceDiagram\n  title Compra\n  A->>B: x");

    assert_equivalent(&a, &b);
    assert_eq!(b.meta.title, "Compra");
}

#[test]
fn conserva_los_saltos_de_linea_de_un_texto() {
    let (_, b) = round_trip("sequenceDiagram\n  A->>B: primera<br/>segunda");

    assert_eq!(script(&b).messages()[0].text, "primera\nsegunda");
}

#[test]
fn un_texto_con_dos_puntos_sobrevive() {
    let (_, b) = round_trip("sequenceDiagram\n  A->>B: puerto 443: TLS");

    assert_eq!(script(&b).messages()[0].text, "puerto 443: TLS");
}

#[test]
fn un_documento_vacio_genera_solo_la_cabecera() {
    let mut doc = Document::new("d", DiagramKind::Sequence, 0);
    doc.script = Some(SequenceScript::default());

    assert_eq!(emit(&doc), "sequenceDiagram\n");
}

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    let source = "sequenceDiagram\n\
         \x20 title Compra de un pedido\n\
         \x20 actor Cliente\n\
         \x20 participant web as Portal web\n\
         \x20 participant api as API de pedidos\n\
         \x20 participant pago as Pasarela\n\
         \n\
         \x20 Cliente->>+web: comprar\n\
         \x20 Note over web,api: todo sobre TLS\n\
         \x20 web->>+api: POST /pedidos\n\
         \x20 api->>api: validar carrito\n\
         \x20 alt pago autorizado\n\
         \x20   api->>+pago: cobrar\n\
         \x20   pago-->>-api: autorizado\n\
         \x20   api-->>-web: 201 Created\n\
         \x20 else rechazado\n\
         \x20   api-->>web: 402 Rechazado\n\
         \x20 end\n\
         \x20 web-->>-Cliente: confirmacion";

    let (a, b) = round_trip(source);
    assert_equivalent(&a, &b);
}
