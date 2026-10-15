use super::*;
use crate::model::SequenceScript;

fn parse(source: &str) -> ImportResult {
    import(source, 0)
}

fn ok(source: &str) -> Document {
    let result = parse(source);
    assert!(
        result.ok(),
        "diagnosticos inesperados: {:#?}",
        result.diagnostics
    );
    result.document
}

/// Importa un cuerpo, poniendole el armazon.
fn body(source: &str) -> Document {
    ok(&format!("@startuml\n{source}\n@enduml"))
}

fn codes(result: &ImportResult) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn script(doc: &Document) -> &SequenceScript {
    doc.script.as_ref().expect("falta el guion")
}

fn names(doc: &Document) -> Vec<&str> {
    doc.nodes.iter().map(|n| n.label.as_str()).collect()
}

fn messages(doc: &Document) -> Vec<&Message> {
    script(doc).messages()
}

/* --- Participantes -------------------------------------------------------- */

#[test]
fn el_diagrama_importado_es_de_secuencia_y_sin_geometria() {
    let doc = body("Alice -> Bob : hola");

    assert_eq!(doc.kind, DiagramKind::Sequence);
    assert!(doc.script.is_some());
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn reconoce_las_palabras_de_cada_forma() {
    let doc = body(
        "actor Cliente\n\
         participant Portal\n\
         database Base\n\
         queue Cola\n\
         control Mando",
    );

    assert_eq!(doc.nodes[0].shape, NodeShape::Actor);
    assert_eq!(doc.nodes[1].shape, NodeShape::Rectangle);
    assert_eq!(doc.nodes[2].shape, NodeShape::Cylinder);
    assert_eq!(doc.nodes[3].shape, NodeShape::Hexagon);
    assert_eq!(doc.nodes[4].shape, NodeShape::Ellipse);
}

#[test]
fn un_alias_pone_la_etiqueta_y_los_dos_nombres_sirven() {
    let doc = body(
        "participant \"API de pedidos\" as api\n\
         api -> api : validar\n\
         \"API de pedidos\" -> api : otra",
    );

    assert_eq!(names(&doc), ["API de pedidos"]);
    assert_eq!(messages(&doc).len(), 2);
}

#[test]
fn declarar_fija_el_orden_de_izquierda_a_derecha() {
    let doc = body("participant Z\nparticipant A\nA -> Z : x");

    assert_eq!(names(&doc), ["Z", "A"]);
}

#[test]
fn mencionar_sin_declarar_tambien_crea_al_participante() {
    let doc = body("Alice -> Bob : hola\nBob -> Carol : adios");

    assert_eq!(names(&doc), ["Alice", "Bob", "Carol"]);
}

#[test]
fn lee_el_titulo() {
    assert_eq!(body("title Compra\nA -> B : x").meta.title, "Compra");
}

#[test]
fn se_salta_los_comentarios() {
    let doc = body("' esto no cuenta\nA -> B : hola\n/' ni\n esto '/\nB -> A : vale");

    assert_eq!(messages(&doc).len(), 2);
}

/* --- Mensajes ------------------------------------------------------------- */

#[test]
fn reconoce_los_tipos_de_flecha() {
    let doc = body("A -> B : llamada\nB --> A : retorno\nA ->> B : asincrono");

    let kinds: Vec<_> = messages(&doc).iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        [MessageKind::Sync, MessageKind::Reply, MessageKind::Async]
    );
}

#[test]
fn una_flecha_al_reves_invierte_el_mensaje() {
    let doc = body("participant A\nparticipant B\nB <- A : empujar");

    // `B <- A` se lee de derecha a izquierda: manda A, recibe B.
    let m = messages(&doc)[0];
    assert_eq!(m.from, doc.nodes[0].id, "manda A");
    assert_eq!(m.to, doc.nodes[1].id, "recibe B");
}

#[test]
fn el_texto_va_detras_de_los_dos_puntos_y_puede_llevar_mas() {
    let doc = body("A -> B : usa el puerto 443: TLS");

    assert_eq!(messages(&doc)[0].text, "usa el puerto 443: TLS");
}

#[test]
fn un_mensaje_sin_texto_vale() {
    let doc = body("A -> B");

    assert_eq!(messages(&doc).len(), 1);
    assert_eq!(messages(&doc)[0].text, "");
}

#[test]
fn convierte_los_saltos_de_linea() {
    let doc = body("A -> B : primera\\nsegunda");

    assert_eq!(messages(&doc)[0].text, "primera\nsegunda");
}

#[test]
fn un_mensaje_a_uno_mismo_se_reconoce() {
    let doc = body("A -> A : pensar");

    assert!(messages(&doc)[0].is_self());
}

#[test]
fn una_flecha_sin_participante_a_un_lado_es_un_error() {
    let result = parse("@startuml\n-> B : hola\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingArrow"]);
}

#[test]
fn una_linea_que_no_se_entiende_se_reporta_con_su_posicion() {
    let result = parse("@startuml\nA -> B : bien\n  ???\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["badStatement"]);

    let span = result.diagnostics[0].span.as_ref().unwrap();
    assert_eq!((span.line, span.column), (3, 3));
}

/* --- Sufijos de ejecucion ------------------------------------------------- */

#[test]
fn los_sufijos_abren_y_cierran_la_ejecucion() {
    let doc = body("A -> B ++ : llamar\nB --> A -- : volver");

    let m = messages(&doc);
    assert!(m[0].activate, "`++` abre la del destinatario");
    assert!(m[1].deactivate, "`--` cierra la del otro extremo");
}

#[test]
fn los_sufijos_se_combinan() {
    let doc = body("A -> B ++ : llamar\nB -> C --++ : seguir");

    let segundo = messages(&doc)[1];
    assert!(segundo.activate);
    assert!(segundo.deactivate);
}

#[test]
fn los_dos_asteriscos_crean_al_participante() {
    let doc = body("A -> B ** : nacer");

    assert_eq!(messages(&doc)[0].kind, MessageKind::Create);
}

#[test]
fn las_dos_exclamaciones_lo_destruyen() {
    let doc = body("A -> B !! : morir");

    assert_eq!(messages(&doc)[0].kind, MessageKind::Destroy);
}

#[test]
fn activate_y_deactivate_sueltos_se_enganchan_al_mensaje_anterior() {
    // PlantUML los escribe detras del mensaje al que se refieren.
    let doc = body("A -> B : llamar\nactivate B\nB --> A : volver\ndeactivate B");

    let m = messages(&doc);
    assert!(m[0].activate);
    assert!(m[1].deactivate);
}

#[test]
fn destroy_suelto_marca_el_mensaje_anterior() {
    let doc = body("A -> B : adios\ndestroy B");

    assert_eq!(messages(&doc)[0].kind, MessageKind::Destroy);
}

#[test]
fn create_suelto_marca_el_mensaje_siguiente() {
    let doc = body("participant A\ncreate B\nA -> B : nacer");

    assert_eq!(messages(&doc)[0].kind, MessageKind::Create);
}

#[test]
fn un_create_sin_su_mensaje_avisa() {
    let result = parse("@startuml\nA -> A : solo\ncreate B\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["danglingCreate"]);
}

#[test]
fn un_activate_sin_mensaje_avisa() {
    let result = parse("@startuml\nparticipant B\nactivate B\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["danglingActivation"]);
}

/* --- Marcos --------------------------------------------------------------- */

#[test]
fn un_alt_con_else_produce_dos_ramas() {
    let doc = body("alt hay stock\n  A -> B : reservar\nelse no hay\n  A -> B : avisar\nend");

    let Step::Block(block) = &script(&doc).steps[0] else {
        panic!("no es un marco");
    };
    assert_eq!(block.kind, BlockKind::Alt);
    assert_eq!(block.branches.len(), 2);
    assert_eq!(block.branches[0].label, "hay stock");
    assert_eq!(block.branches[1].label, "no hay");
}

#[test]
fn reconoce_todos_los_marcos() {
    for (word, expected) in [
        ("opt quiza", BlockKind::Opt),
        ("loop 3 veces", BlockKind::Loop),
        ("par a la vez", BlockKind::Par),
        ("critical importante", BlockKind::Critical),
        ("break si falla", BlockKind::Break),
        ("group etiqueta", BlockKind::Group),
    ] {
        let doc = body(&format!("{word}\n  A -> B : x\nend"));
        let Step::Block(block) = &script(&doc).steps[0] else {
            panic!("`{word}` no abrio marco");
        };
        assert_eq!(block.kind, expected, "{word}");
    }
}

#[test]
fn los_marcos_se_anidan() {
    let doc = body("alt fuera\n  opt dentro\n    A -> B : hondo\n  end\nend");

    let Step::Block(fuera) = &script(&doc).steps[0] else {
        panic!("falta el de fuera");
    };
    let Step::Block(dentro) = &fuera.branches[0].steps[0] else {
        panic!("falta el de dentro");
    };

    assert_eq!(dentro.kind, BlockKind::Opt);
}

#[test]
fn un_marco_sin_cerrar_es_un_error() {
    let result = parse("@startuml\nalt si\n  A -> B : x\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedBlock"]);
}

#[test]
fn un_end_de_mas_es_un_error() {
    let result = parse("@startuml\nA -> B : x\nend\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedEnd"]);
}

/* --- Notas ---------------------------------------------------------------- */

#[test]
fn una_nota_se_pega_donde_dice() {
    let doc = body("participant A\nparticipant B\nnote right of A : ojo");

    let Step::Note(note) = &script(&doc).steps[0] else {
        panic!("no es una nota");
    };
    assert_eq!(note.placement, NotePlacement::RightOf);
    assert_eq!(note.text, "ojo");
}

#[test]
fn una_nota_sobre_dos_participantes_los_guarda_los_dos() {
    let doc = body("participant A\nparticipant B\nnote over A, B : las dos");

    let Step::Note(note) = &script(&doc).steps[0] else {
        panic!("no es una nota");
    };
    assert_eq!(note.participants.len(), 2);
}

#[test]
fn una_nota_de_varias_lineas_se_avisa_y_se_salta_entera() {
    let result = parse(
        "@startuml\n\
         participant A\n\
         note over A\n\
         \x20 texto largo\n\
         \x20 [esto tampoco]\n\
         end note\n\
         A -> A : x\n\
         @enduml",
    );

    assert!(result.ok(), "{:#?}", result.diagnostics);
    assert_eq!(codes(&result), ["ignoredNote"]);
    assert_eq!(messages(&result.document).len(), 1);
}

/* --- Directivas ----------------------------------------------------------- */

#[test]
fn autonumber_y_skinparam_se_avisan_y_se_siguen() {
    let result = parse("@startuml\nautonumber\nskinparam monochrome true\nA -> B : x\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective", "ignoredDirective"]);
    assert_eq!(messages(&result.document).len(), 1);
}

#[test]
fn un_separador_se_avisa_y_se_sigue() {
    let result = parse("@startuml\nA -> B : uno\n== Segunda fase ==\nB -> A : dos\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDivider"]);
    assert_eq!(messages(&result.document).len(), 2);
}

/* --- Diagrama completo ---------------------------------------------------- */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok(r#"
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
"#);

    assert_eq!(doc.meta.title, "Compra de un pedido");
    assert_eq!(
        names(&doc),
        ["Cliente", "Portal web", "API de pedidos", "PostgreSQL"]
    );
    assert_eq!(doc.nodes[0].shape, NodeShape::Actor);
    assert_eq!(doc.nodes[3].shape, NodeShape::Cylinder);

    let steps = &script(&doc).steps;
    // comprar, nota, POST, validar, alt, confirmacion
    assert_eq!(steps.len(), 6);

    let all = messages(&doc);
    assert_eq!(all.len(), 7);
    assert!(all[0].activate);
    assert!(all.last().unwrap().deactivate);
}
