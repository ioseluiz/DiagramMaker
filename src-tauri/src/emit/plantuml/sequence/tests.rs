use super::*;
use crate::model::{DiagramKind, SequenceScript};
use crate::parse::plantuml::sequence as parser;

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

/// Lo mismo, pero entrando por el despachador: comprueba ademas que el
/// codigo generado se sigue reconociendo como una secuencia.
fn round_trip_sniffed(source: &str) -> Document {
    let first = crate::parse::plantuml::import(source, 0);
    assert_eq!(first.document.kind, DiagramKind::Sequence, "no lo olfateo");

    let code = emit(&first.document);
    let second = crate::parse::plantuml::import(&code, 0);

    assert_eq!(
        second.document.kind,
        DiagramKind::Sequence,
        "el codigo generado deja de parecer una secuencia:\n{code}"
    );
    second.document
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

/// Resumen de la estructura del guion, sin ids.
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
    let (a, b) = round_trip("@startuml\nA -> B : pedir\nB --> A : dar\n@enduml");

    assert_equivalent(&a, &b);
}

#[test]
fn el_codigo_generado_es_legible() {
    let imported = parser::import("@startuml\nA -> B : pedir\n@enduml", 0);

    assert_eq!(
        emit(&imported.document),
        "@startuml\n\
         participant \"A\" as n1\n\
         participant \"B\" as n2\n\
         \n\
         n1 -> n2 : pedir\n\
         @enduml\n"
    );
}

#[test]
fn el_codigo_generado_se_sigue_reconociendo_como_secuencia() {
    // Es la trampa del olfateador: si lo que sale no lleva ninguna marca de
    // secuencia, al volver a entrar se leeria como un diagrama de
    // componentes y el documento cambiaria de tipo.
    let doc = round_trip_sniffed("@startuml\nAlice -> Bob : Hola\n@enduml");

    assert_eq!(script(&doc).messages().len(), 1);
}

#[test]
fn conserva_los_tipos_de_mensaje() {
    let (a, b) = round_trip(
        "@startuml\n\
         A -> B : llamada\n\
         B --> A : retorno\n\
         A ->> B : asincrono\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_las_formas_de_los_participantes() {
    let (a, b) = round_trip(
        "@startuml\n\
         actor Cliente\n\
         participant Portal\n\
         database Base\n\
         queue Cola\n\
         control Mando\n\
         Cliente -> Portal : x\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_el_orden_de_los_participantes() {
    let (a, b) = round_trip("@startuml\nparticipant Z\nparticipant A\nA -> Z : x\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(
        b.nodes.iter().map(|n| n.label.as_str()).collect::<Vec<_>>(),
        ["Z", "A"]
    );
}

#[test]
fn conserva_las_barras_de_ejecucion() {
    let (a, b) = round_trip("@startuml\nA -> B ++ : llamar\nB --> A -- : volver\n@enduml");

    assert_equivalent(&a, &b);
    assert!(script(&b).messages()[0].activate);
    assert!(script(&b).messages()[1].deactivate);
}

#[test]
fn un_mensaje_que_abre_y_cierra_cabe_en_una_linea() {
    // En PlantUML los dos sufijos se escriben juntos, al contrario que en
    // Mermaid, que solo admite uno.
    let (a, b) = round_trip("@startuml\nA -> B ++ : llamar\nB -> C --++ : seguir\n@enduml");

    assert_equivalent(&a, &b);
    let segundo = script(&b).messages()[1].clone();
    assert!(segundo.activate);
    assert!(segundo.deactivate);
}

#[test]
fn conserva_los_marcos_y_sus_ramas() {
    let (a, b) = round_trip(
        "@startuml\n\
         alt hay stock\n\
         \x20 A -> B : reservar\n\
         else no hay\n\
         \x20 A -> B : avisar\n\
         end\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_todos_los_tipos_de_marco() {
    for word in [
        "opt quiza",
        "loop 3 veces",
        "par a la vez",
        "critical importante",
        "break si falla",
        "group etiqueta",
    ] {
        let (a, b) = round_trip(&format!("@startuml\n{word}\n  A -> B : x\nend\n@enduml"));
        assert_equivalent(&a, &b);
    }
}

#[test]
fn conserva_los_marcos_anidados() {
    let (a, b) = round_trip(
        "@startuml\n\
         alt fuera\n\
         \x20 opt dentro\n\
         \x20   A -> B : hondo\n\
         \x20 end\n\
         end\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_las_notas() {
    let (a, b) = round_trip(
        "@startuml\n\
         participant A\n\
         participant B\n\
         note right of A : una\n\
         note over A, B : las dos\n\
         @enduml",
    );

    assert_equivalent(&a, &b);
}

#[test]
fn conserva_la_creacion_y_la_destruccion() {
    let (a, b) = round_trip(
        "@startuml\n\
         participant A\n\
         A -> A : arrancar\n\
         create B\n\
         A -> B : nacer\n\
         A -> B !! : morir\n\
         @enduml",
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
    let imported = parser::import(
        "@startuml\nparticipant A\ncreate B\nA -> B : nacer\n@enduml",
        0,
    );
    let code = emit(&imported.document);

    assert_eq!(code.matches("as n2").count(), 1, "{code}");
    assert!(code.contains("create participant \"B\" as n2"), "{code}");
}

#[test]
fn conserva_el_titulo() {
    let (a, b) = round_trip("@startuml\ntitle Compra\nA -> B : x\n@enduml");

    assert_equivalent(&a, &b);
    assert_eq!(b.meta.title, "Compra");
}

#[test]
fn conserva_los_saltos_de_linea_de_un_texto() {
    let (_, b) = round_trip("@startuml\nA -> B : primera\\nsegunda\n@enduml");

    assert_eq!(script(&b).messages()[0].text, "primera\nsegunda");
}

#[test]
fn un_texto_con_dos_puntos_sobrevive() {
    let (_, b) = round_trip("@startuml\nA -> B : puerto 443: TLS\n@enduml");

    assert_eq!(script(&b).messages()[0].text, "puerto 443: TLS");
}

#[test]
fn un_documento_vacio_genera_solo_el_armazon() {
    let mut doc = Document::new("d", DiagramKind::Sequence, 0);
    doc.script = Some(SequenceScript::default());

    assert_eq!(emit(&doc), "@startuml\n@enduml\n");
}

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    let source = r#"
@startuml
title Compra de un pedido
actor Cliente
participant "Portal web" as web
participant "API de pedidos" as api
database "PostgreSQL" as base

Cliente -> web ++ : comprar
note over web, api : todo sobre TLS
web -> api ++ : POST /pedidos
api -> api : validar carrito

alt pago autorizado
  api -> base : reservar
  api --> web -- : 201 Created
else rechazado
  api --> web : 402 Rechazado
end

web --> Cliente -- : confirmacion
@enduml
"#;

    let (a, b) = round_trip(source);
    assert_equivalent(&a, &b);
}
