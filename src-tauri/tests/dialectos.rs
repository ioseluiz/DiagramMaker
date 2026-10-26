//! Lo que promete `docs/dialectos.md`.
//!
//! Ese documento dice donde esta la frontera de cada importador. Una frontera
//! escrita en prosa se desplaza sola: alguien amplia un parser y no actualiza
//! el texto, o peor, alguien lee el texto y cuenta con algo que nunca
//! funciono.
//!
//! Aqui cada afirmacion del documento tiene su linea. Lo que dice que entra,
//! entra sin diagnosticos; lo que dice que no entra, avisa o falla, pero
//! nunca se traga en silencio.

use diagrammaker_lib::parse::{self, SourceLanguage};

/// Importa y devuelve los codigos de diagnostico que salgan.
fn codigos(language: SourceLanguage, source: &str) -> Vec<String> {
    parse::import(language, source, 0)
        .diagnostics
        .iter()
        .map(|d| d.code.clone())
        .collect()
}

/// Lo que el documento dice que entra: ni un diagnostico.
fn entra(language: SourceLanguage, source: &str) {
    let codigos = codigos(language, source);
    assert!(
        codigos.is_empty(),
        "deberia entrar limpio: {codigos:?}\n{source}"
    );
}

/// Lo que el documento dice que no entra: se dice, no se calla.
fn se_dice(language: SourceLanguage, source: &str) {
    let codigos = codigos(language, source);
    assert!(
        !codigos.is_empty(),
        "deberia avisar y no lo hace:\n{source}"
    );
}

/* --- Mermaid: flowchart ----------------------------------------------------- */

#[test]
fn flowchart_entiende_el_estilo_de_un_nodo_concreto() {
    entra(
        SourceLanguage::Mermaid,
        "flowchart LR\n  a[Portal] --> b[API]\n  style a fill:#dbe6fe",
    );
}

#[test]
fn flowchart_avisa_de_los_estilos_por_clase() {
    // No tienen donde guardarse en el documento.
    for linea in ["classDef azul fill:#dbe6fe", "class a azul"] {
        se_dice(
            SourceLanguage::Mermaid,
            &format!("flowchart LR\n  a[Portal] --> b[API]\n  {linea}"),
        );
    }
}

/* --- Mermaid: clases y estados ------------------------------------------------ */

#[test]
fn class_diagram_entiende_estereotipo_estatico_y_abstracto() {
    entra(
        SourceLanguage::Mermaid,
        "classDiagram\n\
         \x20 class Repo {\n\
         \x20   <<interface>>\n\
         \x20   +int id\n\
         \x20   +contar() int$\n\
         \x20   +validar()*\n\
         \x20 }",
    );
}

#[test]
fn state_diagram_entiende_los_tres_estereotipos() {
    entra(
        SourceLanguage::Mermaid,
        "stateDiagram-v2\n\
         \x20 state f <<fork>>\n\
         \x20 state j <<join>>\n\
         \x20 state c <<choice>>\n\
         \x20 [*] --> f\n\
         \x20 f --> c\n\
         \x20 c --> j\n\
         \x20 j --> [*]",
    );
}

/* --- Mermaid: secuencia --------------------------------------------------------- */

#[test]
fn sequence_diagram_entiende_la_activacion_en_su_linea() {
    entra(
        SourceLanguage::Mermaid,
        "sequenceDiagram\n\
         \x20 participant A\n\
         \x20 participant B\n\
         \x20 A->>B: hola\n\
         \x20 activate B\n\
         \x20 B-->>A: adios\n\
         \x20 deactivate B",
    );
}

#[test]
fn sequence_diagram_entiende_la_activacion_pegada_a_la_flecha() {
    // Mermaid la escribe con un signo (`+`), PlantUML con dos (`++`).
    entra(
        SourceLanguage::Mermaid,
        "sequenceDiagram\n\
         \x20 participant A\n\
         \x20 participant B\n\
         \x20 A->>+B: hola\n\
         \x20 B-->>-A: adios",
    );
}

/* --- Mermaid: entidad-relacion -------------------------------------------------- */

#[test]
fn er_diagram_entiende_las_dos_lineas_y_las_marcas() {
    entra(
        SourceLanguage::Mermaid,
        "erDiagram\n\
         \x20 CLIENTE ||--o{ PEDIDO : coloca\n\
         \x20 CLIENTE }|..|{ DIRECCION : usa\n\
         \x20 PEDIDO {\n\
         \x20   int numero PK\n\
         \x20   int cliente_id FK \"a quien pertenece\"\n\
         \x20   string clave UK\n\
         \x20 }",
    );
}

/* --- PlantUML: componentes y casos de uso ---------------------------------------- */

#[test]
fn componentes_entiende_pistas_color_estilo_y_cardinalidades() {
    entra(
        SourceLanguage::PlantUml,
        "@startuml\n\
         [Portal] -down-> [API]\n\
         [API] -[#red]-> [Base]\n\
         [API] -[dashed]-> [Cola]\n\
         [Portal] \"1\" --> \"n\" [API]\n\
         @enduml",
    );
}

#[test]
fn casos_de_uso_entiende_las_dos_formas_cortas() {
    entra(
        SourceLanguage::PlantUml,
        "@startuml\n:Cliente: as c\n(Comprar) as u\nc --> u\n@enduml",
    );
}

/* --- PlantUML: clases ------------------------------------------------------------- */

#[test]
fn clases_entiende_static_y_abstract_entre_llaves() {
    entra(
        SourceLanguage::PlantUml,
        "@startuml\n\
         class Pedido {\n\
         \x20 {static} +contar() : int\n\
         \x20 {abstract} +validar()\n\
         }\n\
         @enduml",
    );
}

/* --- PlantUML: secuencia ----------------------------------------------------------- */

#[test]
fn secuencia_entiende_la_activacion_abreviada() {
    entra(
        SourceLanguage::PlantUml,
        "@startuml\n\
         participant A\n\
         participant B\n\
         A -> B ++ : hola\n\
         B --> A -- : adios\n\
         @enduml",
    );
}

#[test]
fn secuencia_avisa_de_autonumber() {
    se_dice(
        SourceLanguage::PlantUml,
        "@startuml\nautonumber\nA -> B : hola\n@enduml",
    );
}

#[test]
fn secuencia_no_lee_la_desactivacion_sola_en_su_linea() {
    se_dice(
        SourceLanguage::PlantUml,
        "@startuml\nparticipant A\nparticipant B\nA -> B ++ : hola\nB --\n@enduml",
    );
}

/* --- PlantUML: estados -------------------------------------------------------------- */

#[test]
fn estados_avisa_de_las_regiones_concurrentes() {
    se_dice(
        SourceLanguage::PlantUml,
        "@startuml\nstate Activo {\n  A --> B\n  --\n  C --> D\n}\n@enduml",
    );
}

/* --- PlantUML: actividad ------------------------------------------------------------- */

#[test]
fn actividad_entiende_las_estructuras_que_promete() {
    entra(
        SourceLanguage::PlantUml,
        "@startuml\n\
         start\n\
         :Uno;\n\
         if (a?) then (si)\n\
         \x20 :Dos;\n\
         elseif (b?) then (si)\n\
         \x20 :Tres;\n\
         else (no)\n\
         \x20 :Cuatro;\n\
         endif\n\
         while (c?) is (si)\n\
         \x20 :Cinco;\n\
         endwhile (no)\n\
         repeat\n\
         \x20 :Seis;\n\
         repeat while (d?) is (si)\n\
         partition Almacen {\n\
         \x20 :Siete;\n\
         }\n\
         fork\n\
         \x20 :Ocho;\n\
         fork again\n\
         \x20 :Nueve;\n\
         end fork\n\
         split\n\
         \x20 :Diez;\n\
         split again\n\
         \x20 :Once;\n\
         end split\n\
         stop\n\
         @enduml",
    );
}

#[test]
fn actividad_rechaza_la_sintaxis_antigua() {
    se_dice(
        SourceLanguage::PlantUml,
        "@startuml\nstart\n(*) --> \"Primera accion\"\n@enduml",
    );
}

#[test]
fn actividad_avisa_de_lo_que_no_importa() {
    for linea in ["detach", "kill", "backward:Rehacer;", "-> con datos;"] {
        se_dice(
            SourceLanguage::PlantUml,
            &format!("@startuml\nstart\n:Uno;\n{linea}\n:Dos;\nstop\n@enduml"),
        );
    }
}

/* --- DBML ------------------------------------------------------------------------------ */

#[test]
fn dbml_entiende_lo_que_el_documento_promete() {
    entra(
        SourceLanguage::Dbml,
        r#"
Project tienda {
  note: 'Esquema de ejemplo'
}

Enum estado {
  nuevo
  pagado
}

Table clientes {
  id integer [pk, increment]
  correo varchar(160) [unique, not null, note: 'login']
  situacion estado [default: 'nuevo']
}

Table pedidos {
  id integer [pk]
  cliente_id integer [ref: > clientes.id]
}

TableGroup ventas {
  clientes
  pedidos
}
"#,
    );
}

#[test]
fn flowchart_entiende_varios_nodos_con_ampersand() {
    entra(SourceLanguage::Mermaid, "flowchart LR\n  a & b --> c & d");
}

#[test]
fn flowchart_no_parte_un_ampersand_de_una_etiqueta() {
    entra(
        SourceLanguage::Mermaid,
        "flowchart LR\n  a[Ventas & Marketing] --> b",
    );
}

/* --- El texto no es solo ASCII --------------------------------------------- */

#[test]
fn todos_los_idiomas_aceptan_tildes_y_enies() {
    // El recorrido completo esta en `acentos.rs`; aqui queda la promesa de
    // que ninguno de los tres se atraganta con ellas.
    entra(
        SourceLanguage::Dbml,
        "Table artículos {\n  código integer [pk]\n}",
    );
    entra(
        SourceLanguage::Mermaid,
        "flowchart LR\n  Gestión --> b[Añadir artículo]",
    );
    entra(
        SourceLanguage::PlantUml,
        "@startuml\n[Gestión] --> [Añadir artículo]\n@enduml",
    );
}
