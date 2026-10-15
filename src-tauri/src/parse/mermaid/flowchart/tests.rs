use super::*;

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

fn codes(result: &ImportResult) -> Vec<&str> {
    result.diagnostics.iter().map(|d| d.code.as_str()).collect()
}

fn node<'a>(doc: &'a Document, label: &str) -> &'a Node {
    doc.nodes
        .iter()
        .find(|n| n.label == label)
        .unwrap_or_else(|| panic!("no hay ningun nodo `{label}`"))
}

/* --- Cabecera y direccion ------------------------------------------------ */

#[test]
fn reconoce_flowchart_y_graph() {
    assert_eq!(ok("flowchart TD\n  A --> B").nodes.len(), 2);
    assert_eq!(ok("graph LR\n  A --> B").nodes.len(), 2);
}

#[test]
fn el_diagrama_importado_es_de_arquitectura() {
    assert_eq!(ok("flowchart TD\n  A").kind, DiagramKind::Architecture);
}

#[test]
fn lee_las_cuatro_direcciones() {
    for (code, expected) in [
        ("TD", LayoutDirection::Down),
        ("TB", LayoutDirection::Down),
        ("BT", LayoutDirection::Up),
        ("LR", LayoutDirection::Right),
        ("RL", LayoutDirection::Left),
    ] {
        let doc = ok(&format!("flowchart {code}\n  A --> B"));
        assert_eq!(doc.direction, expected, "direccion {code}");
    }
}

#[test]
fn una_direccion_desconocida_avisa_y_sigue() {
    let result = parse("flowchart XY\n  A --> B");

    assert!(result.ok());
    assert_eq!(codes(&result), ["unknownDirection"]);
    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn sin_cabecera_avisa_pero_importa() {
    let result = parse("A --> B");

    assert!(result.ok());
    assert_eq!(codes(&result), ["missingHeader"]);
    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn el_documento_importado_llega_sin_geometria() {
    let doc = ok("flowchart TD\n  A --> B");

    assert!(doc.nodes.iter().all(|n| n.geometry.is_none()));
}

/* --- Formas -------------------------------------------------------------- */

#[test]
fn reconoce_las_formas_por_sus_delimitadores() {
    let doc = ok("flowchart TD\n\
         a[Rectangulo]\n\
         b(Redondeado)\n\
         c([Estadio])\n\
         d[(Cilindro)]\n\
         e((Circulo))\n\
         f{Rombo}\n\
         g{{Hexagono}}\n\
         h[/Paralelogramo/]");

    assert_eq!(node(&doc, "Rectangulo").shape, NodeShape::Rectangle);
    assert_eq!(node(&doc, "Redondeado").shape, NodeShape::RoundedRect);
    assert_eq!(node(&doc, "Estadio").shape, NodeShape::RoundedRect);
    assert_eq!(node(&doc, "Cilindro").shape, NodeShape::Cylinder);
    assert_eq!(node(&doc, "Circulo").shape, NodeShape::Ellipse);
    assert_eq!(node(&doc, "Rombo").shape, NodeShape::Diamond);
    assert_eq!(node(&doc, "Hexagono").shape, NodeShape::Hexagon);
    assert_eq!(node(&doc, "Paralelogramo").shape, NodeShape::Parallelogram);
}

#[test]
fn los_delimitadores_largos_ganan_a_los_cortos() {
    // Si se probara `(` antes que `([`, un estadio se leeria como
    // redondeado y sobraria un corchete en la etiqueta.
    let doc = ok("flowchart TD\n  a([Estadio])");

    assert_eq!(node(&doc, "Estadio").shape, NodeShape::RoundedRect);
    assert_eq!(doc.nodes.len(), 1);
}

#[test]
fn un_nodo_sin_forma_usa_su_id_como_etiqueta() {
    let doc = ok("flowchart TD\n  servicio --> base");

    assert_eq!(node(&doc, "servicio").shape, NodeShape::Rectangle);
}

#[test]
fn quita_las_comillas_de_una_etiqueta() {
    let doc = ok("flowchart TD\n  a[\"Con espacios y, comas\"]");

    assert_eq!(doc.nodes[0].label, "Con espacios y, comas");
}

#[test]
fn convierte_los_saltos_de_linea_de_html() {
    let doc = ok("flowchart TD\n  a[Primera<br/>Segunda]");

    assert_eq!(doc.nodes[0].label, "Primera\nSegunda");
}

#[test]
fn declarar_la_forma_despues_de_usar_el_nodo_la_aplica() {
    // Es muy corriente escribir las relaciones primero y las formas despues.
    let doc = ok("flowchart TD\n  a --> b\n  a[(Base)]");

    assert_eq!(node(&doc, "Base").shape, NodeShape::Cylinder);
    assert_eq!(doc.nodes.len(), 2);
}

/* --- Relaciones ---------------------------------------------------------- */

#[test]
fn reconoce_los_tipos_de_flecha() {
    let doc = ok("flowchart LR\n\
         a --> b\n\
         c --- d\n\
         e -.-> f\n\
         g ==> h");

    assert_eq!(doc.edges[0].kind, EdgeKind::Directed);
    assert_eq!(doc.edges[1].kind, EdgeKind::Association);
    assert_eq!(doc.edges[2].kind, EdgeKind::Dependency);
    assert_eq!(doc.edges[3].kind, EdgeKind::Directed);
}

#[test]
fn la_flecha_punteada_y_la_gruesa_se_marcan_en_el_estilo() {
    let doc = ok("flowchart LR\n  a -.- b\n  c ==> d");

    assert_eq!(doc.edges[0].style.dashed, Some(true));
    assert_eq!(doc.edges[1].style.stroke_width, Some(2.5));
}

#[test]
fn lee_la_etiqueta_entre_barras() {
    let doc = ok("flowchart LR\n  a -->|consulta| b");

    assert_eq!(doc.edges[0].label.as_deref(), Some("consulta"));
}

#[test]
fn lee_la_etiqueta_entre_guiones() {
    let doc = ok("flowchart LR\n  a -- consulta --> b");

    assert_eq!(doc.edges[0].label.as_deref(), Some("consulta"));
    assert_eq!(doc.edges[0].kind, EdgeKind::Directed);
    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn una_cadena_declara_varias_relaciones() {
    let doc = ok("flowchart LR\n  a --> b --> c --> d");

    assert_eq!(doc.nodes.len(), 4);
    assert_eq!(doc.edges.len(), 3);
}

#[test]
fn una_cadena_con_formas_por_el_camino() {
    let doc = ok("flowchart LR\n  a[Web] --> b{{Cola}} --> c[(Base)]");

    assert_eq!(node(&doc, "Cola").shape, NodeShape::Hexagon);
    assert_eq!(doc.edges.len(), 2);
}

#[test]
fn una_flecha_dentro_de_una_etiqueta_no_parte_la_linea() {
    let doc = ok("flowchart LR\n  a[entrada --> salida] --> b");

    assert_eq!(doc.nodes.len(), 2);
    assert_eq!(doc.edges.len(), 1);
    assert_eq!(doc.nodes[0].label, "entrada --> salida");
}

/* --- Subgraph ------------------------------------------------------------ */

#[test]
fn un_subgraph_se_convierte_en_contenedor() {
    let doc = ok("flowchart TD\n\
         subgraph backend\n\
           api --> base\n\
         end\n\
         web --> api");

    let group = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .expect("falta el grupo");

    assert_eq!(group.label, "backend");
    assert_eq!(doc.children_of(Some(&group.id)).count(), 2);
}

#[test]
fn un_subgraph_con_titulo_entre_corchetes() {
    let doc = ok("flowchart TD\n  subgraph api[Capa de servicio]\n    a\n  end");

    let group = doc
        .nodes
        .iter()
        .find(|n| n.shape == NodeShape::Package)
        .unwrap();
    assert_eq!(group.label, "Capa de servicio");
}

#[test]
fn los_subgraph_se_anidan() {
    let doc = ok("flowchart TD\n\
         subgraph fuera\n\
           subgraph dentro\n\
             a\n\
           end\n\
         end");

    let fuera = doc.nodes.iter().find(|n| n.label == "fuera").unwrap();
    let dentro = doc.nodes.iter().find(|n| n.label == "dentro").unwrap();

    assert_eq!(dentro.parent.as_deref(), Some(fuera.id.as_str()));
    assert_eq!(node(&doc, "a").parent.as_deref(), Some(dentro.id.as_str()));
}

#[test]
fn un_subgraph_vacio_no_llega_al_documento() {
    // Un contenedor sin nada dentro no aporta y estorba al auto-layout.
    let doc = ok("flowchart TD\n  subgraph vacio\n  end\n  a --> b");

    assert!(doc.nodes.iter().all(|n| n.shape != NodeShape::Package));
}

#[test]
fn un_subgraph_sin_cerrar_se_reporta() {
    let result = parse("flowchart TD\n  subgraph backend\n    a --> b");

    assert_eq!(codes(&result), ["unterminatedSubgraph"]);
}

#[test]
fn un_end_de_mas_se_reporta() {
    let result = parse("flowchart TD\n  a --> b\n  end");

    assert_eq!(codes(&result), ["unexpectedEnd"]);
}

/* --- Comentarios, estilos y errores -------------------------------------- */

#[test]
fn ignora_los_comentarios() {
    let doc = ok("flowchart TD\n  %% esto no cuenta\n  a --> b %% ni esto");

    assert_eq!(doc.nodes.len(), 2);
}

#[test]
fn un_porcentaje_entre_comillas_no_es_comentario() {
    let doc = ok("flowchart TD\n  a[\"100%% seguro\"]");

    assert!(doc.nodes[0].label.contains("100"));
}

#[test]
fn style_pinta_el_nodo() {
    let doc = ok("flowchart TD\n  a[Web]\n  style a fill:#f9f,stroke:#333");

    assert_eq!(doc.nodes[0].style.fill.as_deref(), Some("#f9f"));
    assert_eq!(doc.nodes[0].style.stroke.as_deref(), Some("#333"));
}

#[test]
fn style_sobre_un_nodo_inexistente_avisa() {
    let result = parse("flowchart TD\n  a\n  style fantasma fill:#f9f");

    assert!(result.ok());
    assert_eq!(codes(&result), ["unknownStyleTarget"]);
}

#[test]
fn classdef_avisa_de_que_no_se_conserva() {
    let result = parse("flowchart TD\n  classDef roja fill:#f00\n  a");

    assert!(result.ok());
    assert_eq!(codes(&result), ["ignoredDirective"]);
}

#[test]
fn una_flecha_sin_nodo_a_un_lado_se_reporta() {
    let result = parse("flowchart TD\n  --> b");

    assert!(!result.ok());
    assert!(codes(&result).contains(&"badStatement"));
}

#[test]
fn un_error_en_una_linea_no_impide_leer_las_siguientes() {
    let result = parse("flowchart TD\n  --> roto\n  a --> b");

    assert_eq!(result.document.nodes.len(), 2);
}

#[test]
fn una_entrada_vacia_produce_un_documento_vacio() {
    let result = parse("");

    assert!(result.ok());
    assert!(result.document.nodes.is_empty());
}

/* --- El documento resultante es coherente -------------------------------- */

#[test]
fn el_documento_importado_pasa_la_validacion() {
    let result = parse(
        "flowchart TD\n\
         subgraph backend\n\
           api[API] --> base[(PostgreSQL)]\n\
         end\n\
         web([Portal]) --> api",
    );

    let report = crate::model::validate(&result.document);
    assert!(report.ok, "validacion fallida: {:#?}", report.diagnostics);
}

#[test]
fn los_ids_de_nodo_son_unicos() {
    let doc = ok("flowchart TD\n  a --> b\n  b --> c\n  a --> c");

    let mut ids: Vec<&str> = doc.nodes.iter().map(|n| n.id.as_str()).collect();
    let total = ids.len();
    ids.sort_unstable();
    ids.dedup();

    assert_eq!(ids.len(), total);
}

#[test]
fn un_diagrama_realista_se_importa_entero() {
    let source = r#"
flowchart LR
  %% Arquitectura de la tienda
  usuario([Cliente])

  subgraph frontal[Capa publica]
    web[Portal web]
    movil[App movil]
  end

  subgraph servicios[Servicios]
    api[API REST]
    cola{{Cola de eventos}}
  end

  base[(PostgreSQL)]
  pagos[/Pasarela de pagos/]

  usuario --> web
  usuario --> movil
  web -- HTTPS --> api
  movil -- HTTPS --> api
  api --> base
  api -.-> cola
  api ==>|cobro| pagos

  style pagos fill:#fde68a
"#;

    let result = parse(source);
    assert!(result.ok(), "diagnosticos: {:#?}", result.diagnostics);

    let doc = result.document;
    assert_eq!(doc.direction, LayoutDirection::Right);
    // 7 nodos + 2 grupos
    assert_eq!(doc.nodes.len(), 9);
    assert_eq!(doc.edges.len(), 7);
    assert_eq!(node(&doc, "PostgreSQL").shape, NodeShape::Cylinder);
    assert_eq!(node(&doc, "Cola de eventos").shape, NodeShape::Hexagon);
    assert_eq!(
        node(&doc, "Pasarela de pagos").style.fill.as_deref(),
        Some("#fde68a")
    );
    assert!(crate::model::validate(&doc).ok);
}
