use super::*;

use crate::model::DiagramKind;
use crate::parse::plantuml as parser;

fn import(source: &str) -> Document {
    let result = parser::import(source, 0);
    assert!(
        result.ok(),
        "la importacion fallo: {:#?}",
        result.diagnostics
    );
    assert_eq!(result.document.kind, DiagramKind::Activity);
    result.document
}

/// Importa un cuerpo, poniendole el armazon.
fn doc(source: &str) -> Document {
    import(&format!("@startuml\n{source}\n@enduml"))
}

/// El codigo generado, sin el armazon ni las lineas en blanco.
fn code(doc: &Document) -> String {
    emit(doc)
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('@'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Lo que de verdad hay que sostener: importar, generar y volver a importar
/// tiene que dar el mismo grafo.
///
/// Se compara por forma y texto, no por id: los ids se vuelven a repartir en
/// cada importacion y no significan nada.
fn round_trip(source: &str) -> (Document, Document) {
    let first = doc(source);
    let generated = emit(&first);
    let second = parser::import(&generated, 0);

    assert!(
        second.ok(),
        "el codigo generado no vuelve a entrar:\n{generated}\n{:#?}",
        second.diagnostics
    );

    assert_eq!(
        summary(&first),
        summary(&second.document),
        "distinto grafo.\nGenerado:\n{generated}"
    );

    (first, second.document)
}

fn name(doc: &Document, id: &str) -> String {
    match doc.node(id) {
        Some(node) if node.label.is_empty() => format!("{:?}", node.shape),
        Some(node) => node.label.replace('\n', " | "),
        None => "?".into(),
    }
}

/// Resumen del grafo sin ids: nodos con su forma y su contenedor, y aristas
/// con su etiqueta. Ordenado, porque el orden de creacion si puede cambiar.
fn summary(doc: &Document) -> Vec<String> {
    let mut out: Vec<String> = doc
        .nodes
        .iter()
        .map(|n| {
            format!(
                "nodo {} {:?} en {}",
                name(doc, &n.id),
                n.shape,
                n.parent
                    .as_deref()
                    .map(|p| name(doc, p))
                    .unwrap_or_else(|| "raiz".into())
            )
        })
        .collect();
    out.sort();

    let mut edges: Vec<String> = doc
        .edges
        .iter()
        .map(|e| {
            format!(
                "arista {} -> {} : {}",
                name(doc, &e.source.node),
                name(doc, &e.target.node),
                e.label.clone().unwrap_or_default()
            )
        })
        .collect();
    edges.sort();
    out.extend(edges);

    out
}

/* --- Lo basico ------------------------------------------------------------- */

#[test]
fn una_secuencia_sale_en_el_mismo_orden() {
    assert_eq!(
        code(&doc("start\n:Uno;\n:Dos;\nstop")),
        "start\n:Uno;\n:Dos;\nstop"
    );
}

#[test]
fn un_documento_vacio_genera_solo_el_armazon() {
    let empty = Document::new("d", DiagramKind::Activity, 0);

    assert_eq!(emit(&empty), "@startuml\n\n@enduml\n");
}

#[test]
fn una_accion_de_varias_lineas_se_escribe_en_varias_lineas() {
    let generated = code(&doc(":Primera\nsegunda;"));

    assert_eq!(generated, ":Primera\nsegunda;");
}

#[test]
fn conserva_el_titulo() {
    assert!(emit(&doc("title Flujo del pedido\n:A;")).contains("title Flujo del pedido"));
}

#[test]
fn lo_que_va_detras_de_un_final_se_escribe_detras() {
    // `stop` corta el flujo; lo que sigue es otro trozo suelto, y asi sale.
    round_trip("start\n:A;\nstop\n:Suelta;");
}

/* --- Condiciones ----------------------------------------------------------- */

#[test]
fn una_condicion_vuelve_a_salir_con_sus_dos_ramas() {
    assert_eq!(
        code(&doc(
            "if (hay stock?) then (si)\n  :Reservar;\nelse (no)\n  :Avisar;\nendif"
        )),
        "if (hay stock?) then (si)\n  :Reservar;\nelse (no)\n  :Avisar;\nendif"
    );
}

#[test]
fn sin_else_no_se_escribe_un_else_vacio() {
    assert_eq!(
        code(&doc("if (c) then (si)\n  :A;\nendif\n:Comun;")),
        "if (c) then (si)\n  :A;\nendif\n:Comun;"
    );
}

#[test]
fn un_elseif_se_recupera_como_elseif_y_no_como_if_anidado() {
    assert_eq!(
        code(&doc("if (a?) then (si)\n\
             \x20 :Uno;\n\
             elseif (b?) then (si)\n\
             \x20 :Dos;\n\
             else (no)\n\
             \x20 :Tres;\n\
             endif")),
        "if (a?) then (si)\n  :Uno;\nelseif (b?) then (si)\n  :Dos;\nelse (no)\n  :Tres;\nendif"
    );
}

#[test]
fn una_rama_que_acaba_en_final_sobrevive() {
    round_trip("start\nif (c) then (si)\n  stop\nelse (no)\n  :B;\nendif\n:Comun;\nstop");
}

#[test]
fn las_condiciones_anidadas_sobreviven() {
    round_trip(
        "if (a?) then (si)\n\
         \x20 if (b?) then (si)\n\
         \x20   :Dentro;\n\
         \x20 endif\n\
         endif\n\
         :Fin;",
    );
}

/* --- Bucles ---------------------------------------------------------------- */

#[test]
fn un_while_vuelve_a_salir_como_while() {
    assert_eq!(
        code(&doc(
            "while (quedan?) is (si)\n  :Procesar;\nendwhile (no)\n:Cerrar;"
        )),
        "while (quedan?) is (si)\n  :Procesar;\nendwhile (no)\n:Cerrar;"
    );
}

#[test]
fn un_repeat_vuelve_a_salir_como_repeat() {
    // Es lo que separa a los dos bucles: si esto saliera como `while`, el
    // cuerpo dejaria de correr siempre una vez.
    assert_eq!(
        code(&doc(
            "repeat\n  :Revisar;\n  :Anotar;\nrepeat while (faltan?) is (si)\n:Cerrar;"
        )),
        "repeat\n  :Revisar;\n  :Anotar;\nrepeat while (faltan?) is (si)\n:Cerrar;"
    );
}

#[test]
fn una_condicion_dentro_de_un_while_sobrevive() {
    round_trip(
        "while (quedan?) is (si)\n\
         \x20 if (valida?) then (si)\n\
         \x20   :Aceptar;\n\
         \x20 else (no)\n\
         \x20   :Rechazar;\n\
         \x20 endif\n\
         endwhile\n\
         :Fin;",
    );
}

#[test]
fn un_while_dentro_de_un_while_sobrevive() {
    round_trip(
        "while (a?) is (si)\n\
         \x20 while (b?) is (si)\n\
         \x20   :Dentro;\n\
         \x20 endwhile\n\
         endwhile\n\
         :Fin;",
    );
}

#[test]
fn un_repeat_dentro_de_un_while_sobrevive() {
    round_trip(
        "while (a?) is (si)\n\
         \x20 repeat\n\
         \x20   :Dentro;\n\
         \x20 repeat while (b?) is (si)\n\
         endwhile\n\
         :Fin;",
    );
}

/* --- Ramas paralelas ------------------------------------------------------- */

#[test]
fn un_fork_vuelve_a_salir_con_sus_dos_barras() {
    assert_eq!(
        code(&doc(
            "fork\n  :Correo;\nfork again\n  :Inventario;\nend fork\n:Seguir;"
        )),
        "fork\n  :Correo;\nfork again\n  :Inventario;\nend fork\n:Seguir;"
    );
}

#[test]
fn end_merge_vuelve_a_salir_como_end_merge() {
    assert_eq!(
        code(&doc("fork\n  :A;\nfork again\n  :B;\nend merge\n:Seguir;")),
        "fork\n  :A;\nfork again\n  :B;\nend merge\n:Seguir;"
    );
}

#[test]
fn un_split_vuelve_a_salir_como_split() {
    assert_eq!(
        code(&doc(
            ":Antes;\nsplit\n  :A;\nsplit again\n  :B;\nend split\n:Despues;"
        )),
        ":Antes;\nsplit\n  :A;\nsplit again\n  :B;\nend split\n:Despues;"
    );
}

#[test]
fn un_fork_de_tres_ramas_sobrevive() {
    round_trip("fork\n  :A;\nfork again\n  :B;\nfork again\n  :C;\nend fork\n:Fin;");
}

#[test]
fn una_condicion_dentro_de_una_rama_paralela_sobrevive() {
    round_trip(
        "fork\n\
         \x20 if (c) then (si)\n\
         \x20   :A;\n\
         \x20 else (no)\n\
         \x20   :B;\n\
         \x20 endif\n\
         fork again\n\
         \x20 :C;\n\
         end fork\n\
         :Fin;",
    );
}

/* --- Particiones ----------------------------------------------------------- */

#[test]
fn una_particion_vuelve_a_salir_como_particion() {
    assert_eq!(
        code(&doc(
            ":Antes;\npartition \"Centro de reparto\" {\n  :Dentro;\n}\n:Despues;"
        )),
        ":Antes;\npartition \"Centro de reparto\" {\n  :Dentro;\n}\n:Despues;"
    );
}

#[test]
fn las_particiones_anidadas_sobreviven() {
    round_trip("partition Fuera {\n  :A;\n  partition Dentro {\n    :B;\n  }\n}\n:Fin;");
}

#[test]
fn un_bucle_dentro_de_una_particion_sobrevive() {
    round_trip(
        "partition Almacen {\n\
         \x20 repeat\n\
         \x20   :Preparar;\n\
         \x20 repeat while (faltan?) is (si)\n\
         }\n\
         :Fin;",
    );
}

/* --- Lo que no se puede escribir -------------------------------------------- */

#[test]
fn lo_que_no_cabe_se_dice_en_vez_de_inventarse() {
    use crate::model::{Edge, EdgeKind, Node, NodeShape};

    // Un bucle con dos entradas, dibujado a mano: se entra tanto por
    // `Izquierda` como por `Derecha`, y las dos se llaman entre si. No hay
    // `while` ni `repeat` que diga eso, y escribir el parecido seria peor que
    // decir que falta.
    let mut document = Document::new("d", DiagramKind::Activity, 0);
    for (id, label, shape) in [
        ("c", "por donde?", NodeShape::Diamond),
        ("a", "Izquierda", NodeShape::RoundedRect),
        ("b", "Derecha", NodeShape::RoundedRect),
    ] {
        document.nodes.push(Node::new(id, label, shape));
    }
    for (id, from, to) in [
        ("e1", "c", "a"),
        ("e2", "c", "b"),
        ("e3", "a", "b"),
        ("e4", "b", "a"),
    ] {
        document
            .edges
            .push(Edge::new(id, from, to, EdgeKind::Directed));
    }

    let generated = emit(&document);

    // Ningun paso se pierde...
    assert!(generated.contains("Izquierda"), "{generated}");
    assert!(generated.contains("Derecha"), "{generated}");
    // ...y el que no cupo se nombra.
    assert!(
        generated.contains("' sin escribir: el paso de Derecha a Izquierda"),
        "tiene que decir que no cabe: {generated}"
    );
}

#[test]
fn un_diagrama_que_si_cabe_no_dice_que_falte_nada() {
    let generated = emit(&doc("start
         if (c) then (si)
           :A;
         else (no)
           :B;
         endif
         :Comun;
         stop"));

    assert!(!generated.contains("' sin escribir"), "{generated}");
}

/* --- Diagrama completo ------------------------------------------------------ */

#[test]
fn un_diagrama_realista_sobrevive_la_ida_y_vuelta() {
    round_trip(
        "start\n\
         :Recibir pedido;\n\
         \n\
         if (hay stock?) then (si)\n\
         \x20 :Reservar articulos;\n\
         else (no)\n\
         \x20 :Avisar al cliente;\n\
         \x20 stop\n\
         endif\n\
         \n\
         partition \"Centro de reparto\" {\n\
         \x20 repeat\n\
         \x20   :Preparar linea;\n\
         \x20 repeat while (faltan lineas?) is (si)\n\
         }\n\
         \n\
         fork\n\
         \x20 :Enviar correo;\n\
         fork again\n\
         \x20 :Actualizar inventario;\n\
         end fork\n\
         \n\
         while (hay incidencias?) is (si)\n\
         \x20 :Resolver incidencia;\n\
         endwhile (no)\n\
         \n\
         :Cerrar pedido;\n\
         stop",
    );
}

#[test]
fn el_generador_publico_lleva_la_actividad_a_plantuml() {
    // Entrando por `emit::emit`, que es por donde entra la aplicacion.
    let document = doc("start\n:Hacer algo;\nstop");
    let generated = crate::emit::emit(crate::emit::SourceLanguage::PlantUml, &document);

    assert!(generated.contains("start"), "{generated}");
    assert!(generated.contains(":Hacer algo;"), "{generated}");
}

#[test]
fn una_rama_cortada_se_lleva_dentro_lo_que_seguia() {
    // Decision tomada, no descuido: si una rama acaba en `stop`, las dos no
    // se juntan en ningun sitio y el grafo ya no dice donde cerraba el
    // bloque. Lo que seguia se escribe dentro de la rama viva. El flujo es el
    // mismo —lo comprueba la ida y vuelta de aqui al lado— solo que sangrado.
    assert_eq!(
        code(&doc("if (c) then (si)\n\
             \x20 :Sigue;\n\
             else (no)\n\
             \x20 :Corta;\n\
             \x20 stop\n\
             endif\n\
             :Despues;")),
        "if (c) then (si)\n  :Sigue;\n  :Despues;\nelse (no)\n  :Corta;\n  stop\nendif"
    );

    round_trip("if (c) then (si)\n  :Sigue;\nelse (no)\n  :Corta;\n  stop\nendif\n:Despues;");
}
