use super::*;
use crate::model::SequenceScript;

fn parse(source: &str) -> ImportResult {
    import(source, 0)
}

/// Importa esperando que no haya ningun error.
fn ok(source: &str) -> Document {
    let result = parse(source);
    assert!(
        result.ok(),
        "diagnosticos inesperados: {:#?}",
        result.diagnostics
    );
    result.document
}

/// Importa un cuerpo, poniendole la cabecera.
fn body(source: &str) -> Document {
    ok(&format!("sequenceDiagram\n{source}"))
}

fn codes(result: &ImportResult) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn script(doc: &Document) -> &SequenceScript {
    doc.script.as_ref().expect("falta el guion")
}

/// Nombres de los participantes, en su orden.
fn names(doc: &Document) -> Vec<&str> {
    doc.nodes.iter().map(|n| n.label.as_str()).collect()
}

/// Mensajes del guion, aplanados.
fn messages(doc: &Document) -> Vec<&Message> {
    script(doc).messages()
}

/* --- Cabecera y participantes -------------------------------------------- */

#[test]
fn el_diagrama_importado_es_de_secuencia_y_trae_guion() {
    let doc = body("  A->>B: hola");

    assert_eq!(doc.kind, DiagramKind::Sequence);
    assert!(doc.script.is_some());
}

#[test]
fn los_participantes_no_llegan_con_geometria() {
    // En una secuencia la posicion sale del orden, no de coordenadas.
    let doc = body("  A->>B: hola");

    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn declarar_un_participante_fija_su_orden() {
    let doc = body("  participant B\n  participant A\n  A->>B: hola");

    assert_eq!(names(&doc), ["B", "A"]);
}

#[test]
fn mencionar_un_participante_sin_declararlo_lo_crea() {
    let doc = body("  A->>B: hola\n  B->>C: adios");

    assert_eq!(names(&doc), ["A", "B", "C"]);
}

#[test]
fn un_alias_pone_la_etiqueta_y_el_nombre_sigue_sirviendo() {
    let doc = body("  participant api as API de pedidos\n  api->>api: validar");

    assert_eq!(names(&doc), ["API de pedidos"]);
    assert_eq!(messages(&doc).len(), 1);
}

#[test]
fn un_actor_se_dibuja_como_actor() {
    let doc = body("  actor Cliente\n  participant API\n  Cliente->>API: hola");

    assert_eq!(doc.nodes[0].shape, NodeShape::Actor);
    assert_eq!(doc.nodes[1].shape, NodeShape::Rectangle);
}

#[test]
fn lee_el_titulo() {
    assert_eq!(body("  title Compra\n  A->>B: x").meta.title, "Compra");
}

#[test]
fn se_salta_los_comentarios() {
    let doc = body("  %% esto no cuenta\n  A->>B: hola");

    assert_eq!(messages(&doc).len(), 1);
}

#[test]
fn dos_nombres_que_se_limpian_igual_no_colisionan() {
    let doc = body("  a-b->>c: uno\n  a.b->>c: dos");

    assert_eq!(doc.nodes.len(), 3);
    assert_ne!(doc.nodes[0].id, doc.nodes[1].id);
}

/* --- Mensajes ------------------------------------------------------------ */

#[test]
fn reconoce_los_tipos_de_flecha() {
    let doc = body(
        "  A->>B: llamada\n\
         \x20 B-->>A: retorno\n\
         \x20 A-)B: asincrono\n\
         \x20 A->B: linea\n\
         \x20 A--xB: perdido",
    );

    let kinds: Vec<_> = messages(&doc).iter().map(|m| m.kind).collect();
    assert_eq!(
        kinds,
        [
            MessageKind::Sync,
            MessageKind::Reply,
            MessageKind::Async,
            MessageKind::Sync,
            MessageKind::Async,
        ]
    );
}

#[test]
fn la_flecha_mas_larga_gana_a_la_mas_corta() {
    // Si se probara `->` antes que `-->>`, un retorno se leeria como llamada
    // y sobrarian signos en el nombre del destinatario.
    let doc = body("  A-->>B: retorno");

    assert_eq!(messages(&doc)[0].kind, MessageKind::Reply);
    assert_eq!(names(&doc), ["A", "B"]);
}

#[test]
fn el_texto_va_detras_de_los_dos_puntos_y_puede_llevar_mas() {
    let doc = body("  A->>B: usa el puerto 443: TLS");

    assert_eq!(messages(&doc)[0].text, "usa el puerto 443: TLS");
}

#[test]
fn un_mensaje_sin_texto_vale() {
    let doc = body("  A->>B");

    assert_eq!(messages(&doc).len(), 1);
    assert_eq!(messages(&doc)[0].text, "");
}

#[test]
fn convierte_los_saltos_de_linea_de_html() {
    let doc = body("  A->>B: primera<br/>segunda");

    assert_eq!(messages(&doc)[0].text, "primera\nsegunda");
}

#[test]
fn un_mensaje_a_uno_mismo_se_reconoce() {
    let doc = body("  A->>A: pensar");

    assert!(messages(&doc)[0].is_self());
}

#[test]
fn una_flecha_sin_participante_a_un_lado_es_un_error() {
    let result = parse("sequenceDiagram\n  ->>B: hola");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["danglingArrow"]);
}

#[test]
fn una_linea_que_no_se_entiende_se_reporta_con_su_posicion() {
    let result = parse("sequenceDiagram\n  A->>B: bien\n    ???");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["badStatement"]);

    let span = result.diagnostics[0].span.as_ref().unwrap();
    assert_eq!((span.line, span.column), (3, 5));
}

#[test]
fn un_error_no_se_lleva_por_delante_el_resto() {
    let result = parse("sequenceDiagram\n  ???\n  A->>B: bien");

    assert_eq!(messages(&result.document).len(), 1);
}

/* --- Barras de ejecucion ------------------------------------------------- */

#[test]
fn los_signos_de_la_flecha_abren_y_cierran_la_ejecucion() {
    let doc = body("  A->>+B: llamar\n  B-->>-A: volver");

    let m = messages(&doc);
    assert!(m[0].activate, "`+` tiene que activar el destino");
    assert!(m[1].deactivate, "`-` tiene que cerrar el origen");
}

#[test]
fn activate_y_deactivate_sueltos_se_enganchan_a_su_mensaje() {
    // Mermaid los escribe en su propia linea; el documento los guarda en el
    // mensaje al que se refieren.
    let doc = body("  A->>B: llamar\n  activate B\n  B-->>A: volver\n  deactivate B");

    let m = messages(&doc);
    assert!(m[0].activate);
    assert!(m[1].deactivate);
}

#[test]
fn un_activate_dentro_de_un_marco_encuentra_su_mensaje() {
    let doc = body("  alt si\n    A->>B: llamar\n    activate B\n  end");

    assert!(script(&doc).messages()[0].activate);
}

#[test]
fn un_activate_sin_mensaje_al_que_engancharse_avisa() {
    let result = parse("sequenceDiagram\n  participant B\n  activate B");

    assert!(result.ok());
    assert_eq!(codes(&result), ["danglingActivation"]);
}

/* --- Crear y destruir ---------------------------------------------------- */

#[test]
fn create_marca_el_mensaje_que_crea_al_participante() {
    let doc = body("  A->>A: arrancar\n  create participant B\n  A->>B: nacer");

    let m = messages(&doc);
    assert_eq!(m[0].kind, MessageKind::Sync);
    assert_eq!(m[1].kind, MessageKind::Create);
}

#[test]
fn destroy_marca_el_mensaje_que_lo_destruye() {
    let doc = body("  A->>B: hola\n  destroy B\n  A->>B: adios");

    assert_eq!(messages(&doc)[1].kind, MessageKind::Destroy);
}

#[test]
fn un_create_sin_su_mensaje_avisa() {
    let result = parse("sequenceDiagram\n  A->>A: solo\n  create participant B");

    assert!(result.ok());
    assert_eq!(codes(&result), ["danglingCreate"]);
}

/* --- Marcos -------------------------------------------------------------- */

#[test]
fn un_alt_con_else_produce_dos_ramas() {
    let doc = body("  alt hay stock\n    A->>B: reservar\n  else no hay\n    A->>B: avisar\n  end");

    let steps = &script(&doc).steps;
    assert_eq!(steps.len(), 1);

    let Step::Block(block) = &steps[0] else {
        panic!("no es un marco");
    };
    assert_eq!(block.kind, BlockKind::Alt);
    assert_eq!(block.branches.len(), 2);
    assert_eq!(block.branches[0].label, "hay stock");
    assert_eq!(block.branches[1].label, "no hay");
    assert_eq!(block.branches[0].steps.len(), 1);
}

#[test]
fn reconoce_todos_los_marcos() {
    for (source, expected) in [
        ("opt quiza", BlockKind::Opt),
        ("loop cada hora", BlockKind::Loop),
        ("par a la vez", BlockKind::Par),
        ("critical hacerlo", BlockKind::Critical),
        ("break si falla", BlockKind::Break),
        ("rect rgb(0,0,0)", BlockKind::Group),
    ] {
        let doc = body(&format!("  {source}\n    A->>B: x\n  end"));
        let Step::Block(block) = &script(&doc).steps[0] else {
            panic!("`{source}` no abrio marco");
        };
        assert_eq!(block.kind, expected, "{source}");
    }
}

#[test]
fn un_par_se_divide_con_and() {
    let doc = body("  par uno\n    A->>B: x\n  and dos\n    A->>C: y\n  end");

    let Step::Block(block) = &script(&doc).steps[0] else {
        panic!("no es un marco");
    };
    assert_eq!(block.branches.len(), 2);
    assert_eq!(block.branches[1].label, "dos");
}

#[test]
fn los_marcos_se_anidan() {
    let doc = body("  alt fuera\n    opt dentro\n      A->>B: hondo\n    end\n  end");

    let Step::Block(fuera) = &script(&doc).steps[0] else {
        panic!("falta el de fuera");
    };
    let Step::Block(dentro) = &fuera.branches[0].steps[0] else {
        panic!("falta el de dentro");
    };

    assert_eq!(dentro.kind, BlockKind::Opt);
    assert_eq!(dentro.branches[0].steps.len(), 1);
}

#[test]
fn un_marco_sin_cerrar_es_un_error() {
    let result = parse("sequenceDiagram\n  alt si\n    A->>B: x");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedBlock"]);
}

#[test]
fn un_end_de_mas_es_un_error() {
    let result = parse("sequenceDiagram\n  A->>B: x\n  end");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedEnd"]);
}

#[test]
fn un_else_fuera_de_un_marco_es_un_error() {
    let result = parse("sequenceDiagram\n  A->>B: x\n  else no\n  end");

    // El `end` tambien sobra: sin un `alt` delante, el `else` no abrio nada.
    // Las dos quejas son ciertas y se reportan las dos.
    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedElse", "unexpectedEnd"]);
}

#[test]
fn el_end_de_un_box_no_cierra_un_marco() {
    // Un `box` agrupa participantes y tambien se cierra con `end`; sin
    // contarlos aparte, el `end` del box cerraria el `alt`.
    let result = parse(
        "sequenceDiagram\n\
         \x20 box Servicios\n\
         \x20   participant A\n\
         \x20 end\n\
         \x20 alt si\n\
         \x20   A->>A: x\n\
         \x20 end",
    );

    assert!(result.ok(), "{:#?}", result.diagnostics);
    assert_eq!(codes(&result), ["ignoredBox"]);
    assert_eq!(script(&result.document).steps.len(), 1);
}

/* --- Notas --------------------------------------------------------------- */

#[test]
fn una_nota_se_pega_donde_dice() {
    let doc = body("  participant A\n  participant B\n  Note right of A: ojo");

    let Step::Note(note) = &script(&doc).steps[0] else {
        panic!("no es una nota");
    };
    assert_eq!(note.placement, NotePlacement::RightOf);
    assert_eq!(note.text, "ojo");
    assert_eq!(note.participants.len(), 1);
}

#[test]
fn una_nota_sobre_dos_participantes_los_guarda_los_dos() {
    let doc = body("  participant A\n  participant B\n  Note over A,B: las dos");

    let Step::Note(note) = &script(&doc).steps[0] else {
        panic!("no es una nota");
    };
    assert_eq!(note.placement, NotePlacement::Over);
    assert_eq!(note.participants.len(), 2);
}

#[test]
fn una_nota_admite_espacio_detras_de_la_coma() {
    let doc = body("  participant A\n  participant B\n  Note over A, B: las dos");

    let Step::Note(note) = &script(&doc).steps[0] else {
        panic!("no es una nota");
    };
    assert_eq!(note.participants.len(), 2);
}

#[test]
fn una_nota_a_un_participante_que_no_existe_avisa() {
    let result = parse("sequenceDiagram\n  participant A\n  Note over A,Z: ojo");

    assert!(result.ok());
    assert_eq!(codes(&result), ["unknownParticipant"]);
}

/* --- Directivas que no se guardan ---------------------------------------- */

#[test]
fn autonumber_se_avisa_y_se_sigue() {
    let result = parse("sequenceDiagram\n  autonumber\n  A->>B: x");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective"]);
    assert_eq!(messages(&result.document).len(), 1);
}

/* --- Diagrama completo --------------------------------------------------- */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok("sequenceDiagram\n\
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
         \x20 web-->>-Cliente: confirmacion");

    assert_eq!(doc.meta.title, "Compra de un pedido");
    assert_eq!(
        names(&doc),
        ["Cliente", "Portal web", "API de pedidos", "Pasarela"]
    );
    assert_eq!(doc.nodes[0].shape, NodeShape::Actor);

    let steps = &script(&doc).steps;
    // comprar, nota, POST, validar, alt, confirmacion
    assert_eq!(steps.len(), 6);

    let all = messages(&doc);
    assert_eq!(all.len(), 8);
    assert!(all[0].activate);
    assert!(all.last().unwrap().deactivate);

    let Step::Block(block) = &steps[4] else {
        panic!("falta el alt");
    };
    assert_eq!(block.branches.len(), 2);
    assert_eq!(block.branches[0].steps.len(), 3);
}
