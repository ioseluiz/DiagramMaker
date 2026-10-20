use super::*;

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

fn count(doc: &Document, shape: NodeShape) -> usize {
    doc.nodes.iter().filter(|n| n.shape == shape).count()
}

/// Nombre con el que se reconoce un nodo en las pruebas: su texto, o su forma
/// cuando no lo tiene.
fn name(doc: &Document, id: &str) -> String {
    match doc.node(id) {
        Some(node) if node.label.is_empty() => format!("{:?}", node.shape),
        Some(node) => node.label.replace('\n', " | "),
        None => "?".into(),
    }
}

/// El flujo entero, en el orden en que se construyo.
fn flow(doc: &Document) -> Vec<String> {
    doc.edges
        .iter()
        .map(|edge| {
            let from = name(doc, &edge.source.node);
            let to = name(doc, &edge.target.node);
            match &edge.label {
                Some(label) => format!("{from} -{label}-> {to}"),
                None => format!("{from} -> {to}"),
            }
        })
        .collect()
}

/* --- Lo basico ------------------------------------------------------------- */

#[test]
fn el_diagrama_importado_es_de_actividad_y_llega_sin_geometria() {
    let doc = body("start\n:Hacer algo;\nstop");

    assert_eq!(doc.kind, DiagramKind::Activity);
    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

#[test]
fn una_accion_se_dibuja_redondeada_y_sin_los_signos() {
    let doc = body(":Recibir pedido;");

    assert_eq!(doc.nodes.len(), 1);
    assert_eq!(doc.nodes[0].shape, NodeShape::RoundedRect);
    assert_eq!(doc.nodes[0].label, "Recibir pedido");
}

#[test]
fn el_arranque_y_el_final_tienen_su_forma() {
    let doc = body("start\n:A;\nstop");

    assert_eq!(count(&doc, NodeShape::InitialNode), 1);
    assert_eq!(count(&doc, NodeShape::FinalNode), 1);
}

#[test]
fn end_es_otra_manera_de_escribir_stop() {
    assert_eq!(count(&body("start\n:A;\nend"), NodeShape::FinalNode), 1);
}

#[test]
fn el_orden_del_texto_es_el_del_flujo() {
    // Nadie escribe las flechas: salen de que una sentencia va detras de otra.
    let doc = body("start\n:Uno;\n:Dos;\nstop");

    assert_eq!(
        flow(&doc),
        ["InitialNode -> Uno", "Uno -> Dos", "Dos -> FinalNode",]
    );
}

#[test]
fn despues_de_un_final_no_sigue_nada() {
    // `stop` no deja extremo suelto, asi que la accion de detras arranca sola.
    let doc = body("start\n:A;\nstop\n:Suelta;");

    assert_eq!(doc.edges.len(), 2);
    assert!(!flow(&doc).iter().any(|f| f.contains("Suelta")));
}

#[test]
fn una_accion_puede_ocupar_varias_lineas() {
    let doc = body(":Primera\nsegunda;");

    assert_eq!(doc.nodes[0].label, "Primera\nsegunda");
}

#[test]
fn los_otros_signos_de_cierre_tambien_valen() {
    // `|`, `<`, `>`... cambian la forma de la caja en PlantUML; aqui todas son
    // la misma accion.
    let doc = body(":Entrada/\n:Salida]");

    assert_eq!(count(&doc, NodeShape::RoundedRect), 2);
    assert_eq!(doc.nodes[0].label, "Entrada");
}

#[test]
fn lee_el_titulo() {
    assert_eq!(
        body("title Flujo del pedido\n:A;").meta.title,
        "Flujo del pedido"
    );
}

#[test]
fn se_salta_los_comentarios() {
    assert_eq!(body("' nada\n:A;\n/' ni\nesto '/\n:B;").nodes.len(), 2);
}

/* --- Condiciones ----------------------------------------------------------- */

#[test]
fn una_condicion_es_un_rombo_con_sus_dos_salidas_etiquetadas() {
    let doc = body("if (hay stock?) then (si)\n  :Reservar;\nelse (no)\n  :Avisar;\nendif");

    assert_eq!(count(&doc, NodeShape::Diamond), 1);
    assert_eq!(doc.nodes[0].label, "hay stock?");
    assert_eq!(
        flow(&doc),
        ["hay stock? -si-> Reservar", "hay stock? -no-> Avisar"]
    );
}

#[test]
fn las_dos_ramas_vuelven_a_juntarse_en_lo_que_sigue() {
    let doc = body("if (c) then (si)\n  :A;\nelse (no)\n  :B;\nendif\n:Comun;");

    assert_eq!(flow(&doc).len(), 4);
    assert!(flow(&doc).contains(&"A -> Comun".to_string()));
    assert!(flow(&doc).contains(&"B -> Comun".to_string()));
}

#[test]
fn sin_else_la_salida_no_tomada_es_el_camino_que_sigue() {
    let doc = body("if (c) then (si)\n  :A;\nendif\n:Comun;");

    assert_eq!(flow(&doc), ["c -si-> A", "A -> Comun", "c -> Comun"]);
}

#[test]
fn una_rama_que_acaba_en_stop_no_llega_a_lo_que_sigue() {
    let doc = body("if (c) then (si)\n  stop\nelse (no)\n  :B;\nendif\n:Comun;");

    assert!(flow(&doc).contains(&"B -> Comun".to_string()));
    assert_eq!(
        flow(&doc).iter().filter(|f| f.ends_with("Comun")).count(),
        1,
        "la rama cortada no debe llegar"
    );
}

#[test]
fn un_elseif_cuelga_de_la_salida_que_la_condicion_anterior_no_tomo() {
    let doc = body(
        "if (a?) then (si)\n\
         \x20 :Uno;\n\
         elseif (b?) then (si)\n\
         \x20 :Dos;\n\
         else (no)\n\
         \x20 :Tres;\n\
         endif",
    );

    assert_eq!(count(&doc, NodeShape::Diamond), 2);
    assert_eq!(
        flow(&doc),
        ["a? -si-> Uno", "a? -> b?", "b? -si-> Dos", "b? -no-> Tres",]
    );
}

#[test]
fn else_if_separado_se_lee_igual_que_elseif() {
    let doc = body("if (a?) then\n  :Uno;\nelse if (b?) then\n  :Dos;\nendif");

    assert_eq!(count(&doc, NodeShape::Diamond), 2);
}

#[test]
fn una_condicion_sin_then_tambien_vale() {
    let doc = body("if (c)\n  :A;\nendif");

    assert_eq!(doc.nodes[0].label, "c");
    assert_eq!(flow(&doc), ["c -> A"]);
}

#[test]
fn la_condicion_puede_llevar_parentesis_dentro() {
    let doc = body("if (total (con iva) > 100?) then (si)\n  :A;\nendif");

    assert_eq!(doc.nodes[0].label, "total (con iva) > 100?");
}

#[test]
fn una_condicion_pegada_al_parentesis_se_lee_igual() {
    assert_eq!(body("if(c) then\n  :A;\nendif").nodes[0].label, "c");
}

#[test]
fn las_condiciones_se_anidan() {
    let doc = body(
        "if (a?) then (si)\n\
         \x20 if (b?) then (si)\n\
         \x20   :Dentro;\n\
         \x20 endif\n\
         endif\n\
         :Fin;",
    );

    assert_eq!(count(&doc, NodeShape::Diamond), 2);
    assert!(flow(&doc).contains(&"Dentro -> Fin".to_string()));
    assert!(flow(&doc).contains(&"b? -> Fin".to_string()));
    assert!(flow(&doc).contains(&"a? -> Fin".to_string()));
}

#[test]
fn un_if_sin_cerrar_es_un_error() {
    let result = parse("@startuml\nif (c) then\n  :A;\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedIf"]);
}

#[test]
fn un_if_sin_condicion_es_un_error() {
    let result = parse("@startuml\nif entonces\n  :A;\nendif\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["missingCondition"]);
}

#[test]
fn un_endif_suelto_es_un_error() {
    let result = parse("@startuml\n:A;\nendif\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unexpectedClose"]);
}

/* --- Bucles ---------------------------------------------------------------- */

#[test]
fn un_while_pregunta_antes_y_el_cuerpo_vuelve_a_la_pregunta() {
    let doc = body("while (quedan?) is (si)\n  :Procesar;\nendwhile\n:Cerrar;");

    assert_eq!(count(&doc, NodeShape::Diamond), 1);
    assert_eq!(
        flow(&doc),
        [
            "quedan? -si-> Procesar",
            "Procesar -> quedan?",
            "quedan? -> Cerrar",
        ]
    );
}

#[test]
fn el_endwhile_puede_etiquetar_la_salida() {
    let doc = body("while (quedan?) is (si)\n  :Procesar;\nendwhile (no)\n:Cerrar;");

    assert!(flow(&doc).contains(&"quedan? -no-> Cerrar".to_string()));
}

#[test]
fn un_repeat_pregunta_al_final_y_vuelve_al_principio_del_cuerpo() {
    // La diferencia con `while` es justo esta: el cuerpo corre al menos una
    // vez, asi que la vuelta atras apunta a su primer nodo, no al rombo.
    let doc = body("repeat\n  :Revisar;\n  :Anotar;\nrepeat while (faltan?) is (si)\n:Cerrar;");

    assert_eq!(
        flow(&doc),
        [
            "Revisar -> Anotar",
            "Anotar -> faltan?",
            "faltan? -si-> Revisar",
            "faltan? -> Cerrar",
        ]
    );
}

#[test]
fn un_while_sin_cerrar_es_un_error() {
    let result = parse("@startuml\nwhile (c)\n  :A;\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedWhile"]);
}

#[test]
fn un_repeat_sin_su_condicion_es_un_error() {
    let result = parse("@startuml\nrepeat\n  :A;\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedRepeat"]);
}

#[test]
fn una_condicion_dentro_de_un_bucle_sale_sola() {
    let doc = body(
        "while (quedan?) is (si)\n\
         \x20 if (valida?) then (si)\n\
         \x20   :Aceptar;\n\
         \x20 else (no)\n\
         \x20   :Rechazar;\n\
         \x20 endif\n\
         endwhile",
    );

    // Las dos ramas del `if` cierran el bucle volviendo a la pregunta.
    assert!(flow(&doc).contains(&"Aceptar -> quedan?".to_string()));
    assert!(flow(&doc).contains(&"Rechazar -> quedan?".to_string()));
}

/* --- Ramas paralelas ------------------------------------------------------- */

#[test]
fn un_fork_abre_y_cierra_con_barra_de_sincronizacion() {
    let doc = body("fork\n  :Correo;\nfork again\n  :Inventario;\nend fork\n:Seguir;");

    assert_eq!(count(&doc, NodeShape::Bar), 2);
    assert_eq!(
        flow(&doc),
        [
            "Bar -> Correo",
            "Bar -> Inventario",
            "Correo -> Bar",
            "Inventario -> Bar",
            "Bar -> Seguir",
        ]
    );
}

#[test]
fn end_merge_junta_sin_la_segunda_barra() {
    let doc = body("fork\n  :A;\nfork again\n  :B;\nend merge\n:Seguir;");

    assert_eq!(count(&doc, NodeShape::Bar), 1);
    assert!(flow(&doc).contains(&"A -> Seguir".to_string()));
    assert!(flow(&doc).contains(&"B -> Seguir".to_string()));
}

#[test]
fn un_split_reparte_sin_barras() {
    let doc = body(":Antes;\nsplit\n  :A;\nsplit again\n  :B;\nend split\n:Despues;");

    assert_eq!(count(&doc, NodeShape::Bar), 0);
    assert!(flow(&doc).contains(&"Antes -> A".to_string()));
    assert!(flow(&doc).contains(&"Antes -> B".to_string()));
    assert!(flow(&doc).contains(&"A -> Despues".to_string()));
}

#[test]
fn endfork_pegado_vale_igual() {
    let doc = body("fork\n  :A;\nfork again\n  :B;\nendfork");

    assert_eq!(count(&doc, NodeShape::Bar), 2);
}

#[test]
fn un_fork_sin_cerrar_es_un_error() {
    let result = parse("@startuml\nfork\n  :A;\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedFork"]);
}

/* --- Particiones ----------------------------------------------------------- */

#[test]
fn una_particion_es_un_contenedor_con_lo_suyo_dentro() {
    let doc = body("partition Almacen {\n  :Preparar;\n  :Enviar;\n}");

    let caja = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta la caja de la particion");

    assert_eq!(caja.label, "Almacen");
    assert_eq!(doc.children_of(Some(&caja.id)).count(), 2);
}

#[test]
fn el_flujo_atraviesa_la_particion() {
    let doc = body(":Antes;\npartition Almacen {\n  :Dentro;\n}\n:Despues;");

    assert!(flow(&doc).contains(&"Antes -> Dentro".to_string()));
    assert!(flow(&doc).contains(&"Dentro -> Despues".to_string()));
}

#[test]
fn una_particion_con_nombre_entrecomillado_conserva_los_espacios() {
    let doc = body("partition \"Centro de reparto\" {\n  :A;\n}");

    assert_eq!(doc.nodes[0].label, "Centro de reparto");
}

#[test]
fn una_particion_vacia_se_descarta() {
    assert_eq!(body("partition Vacia {\n}\n:A;").nodes.len(), 1);
}

#[test]
fn una_particion_sin_cerrar_es_un_error() {
    let result = parse("@startuml\npartition P {\n  :A;\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["unterminatedPartition"]);
}

/* --- Lo que no entra ------------------------------------------------------- */

#[test]
fn la_sintaxis_antigua_se_rechaza_en_vez_de_interpretarse() {
    let result = parse("@startuml\n(*) --> \"Primera accion\"\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["legacyActivity"]);
}

#[test]
fn una_nota_de_varias_lineas_se_salta_entera() {
    let result = parse(
        "@startuml\n\
         :A;\n\
         note right\n\
         \x20 :B;\n\
         end note\n\
         :C;\n\
         @enduml",
    );

    assert!(result.ok());
    assert_eq!(result.document.nodes.len(), 2, "lo de dentro no cuenta");
}

#[test]
fn detach_y_kill_avisan_y_siguen() {
    let result = parse("@startuml\n:A;\ndetach\n:B;\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDetach"]);
}

#[test]
fn una_etiqueta_suelta_de_flecha_avisa_y_sigue() {
    let result = parse("@startuml\n:A;\n-> con datos;\n:B;\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredArrowLabel"]);
    assert_eq!(result.document.edges.len(), 1);
}

#[test]
fn skinparam_avisa_y_sigue() {
    let result = parse("@startuml\nskinparam monochrome true\n:A;\n@enduml");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective"]);
}

#[test]
fn una_linea_que_no_se_entiende_es_un_error() {
    let result = parse("@startuml\n:A;\nlo que sea\n@enduml");

    assert!(!result.ok());
    assert_eq!(codes(&result), ["badStatement"]);
}

/* --- Diagrama completo ------------------------------------------------------ */

#[test]
fn un_diagrama_realista_entra_entero() {
    let doc = ok(r#"
@startuml
title Flujo del pedido

start
:Recibir pedido;

if (hay stock?) then (si)
  :Reservar articulos;
else (no)
  :Avisar al cliente;
  stop
endif

partition "Centro de reparto" {
  repeat
    :Preparar linea;
  repeat while (faltan lineas?) is (si)
}

fork
  :Enviar correo;
fork again
  :Actualizar inventario;
end fork

while (hay incidencias?) is (si)
  :Resolver incidencia;
endwhile (no)

:Cerrar pedido;
stop
@enduml
"#);

    assert_eq!(doc.meta.title, "Flujo del pedido");
    assert_eq!(count(&doc, NodeShape::InitialNode), 1);
    assert_eq!(count(&doc, NodeShape::FinalNode), 2, "uno por cada salida");
    assert_eq!(count(&doc, NodeShape::Diamond), 3, "if, repeat y while");
    assert_eq!(count(&doc, NodeShape::Bar), 2);
    assert_eq!(count(&doc, NodeShape::Package), 1);

    let reparto = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .unwrap();
    assert_eq!(
        doc.children_of(Some(&reparto.id)).count(),
        2,
        "accion y rombo"
    );

    // Y el flujo cierra: ninguna arista cuelga de un nodo que no existe.
    for edge in &doc.edges {
        assert!(doc.node(&edge.source.node).is_some(), "origen perdido");
        assert!(doc.node(&edge.target.node).is_some(), "destino perdido");
    }
}
