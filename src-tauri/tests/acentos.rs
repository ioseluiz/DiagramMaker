//! El texto llega entero, con tildes y con lo que haga falta.
//!
//! Esto existe por un fallo concreto: el lector de `flowchart` recorria la
//! linea byte a byte y convertia cada uno en un caracter. Una `ó` son dos
//! bytes en UTF-8, asi que salia como `Ã³`; y si la letra caia fuera de unos
//! corchetes, el indice quedaba a mitad de caracter y el siguiente corte de
//! la cadena entraba en panico. Con `panic = "abort"` en el perfil de
//! publicacion, eso tiraba la aplicacion entera.
//!
//! Era de un solo dialecto, pero el sintoma —«no salen bien los acentos»— no
//! distingue de cual. Asi que aqui pasan **todos** los caminos de entrada y
//! de salida, y no solo el que fallo. Un fallo de codificacion en cualquiera
//! de ellos se ve aqui.
//!
//! El texto de prueba lleva a proposito las cuatro longitudes que existen en
//! UTF-8: ASCII de un byte, acentos y `ñ` de dos, un signo de tres y un
//! emoji de cuatro.

use diagrammaker_lib::model::Document;
use diagrammaker_lib::parse::{self, SourceLanguage};
use diagrammaker_lib::{emit, io};

/// Todo el texto del documento, junto.
fn texto(doc: &Document) -> String {
    let mut out = doc.meta.title.clone();

    for node in &doc.nodes {
        out.push('\n');
        out.push_str(&node.label);

        if let Some(table) = node.er() {
            for column in &table.columns {
                out.push('\n');
                out.push_str(&column.name);
                out.push(' ');
                out.push_str(&column.data_type);
            }
        }
        if let Some(class) = node.class() {
            for member in class.attributes.iter().chain(class.operations.iter()) {
                out.push('\n');
                out.push_str(&member.name);
            }
        }
    }

    for edge in &doc.edges {
        if let Some(label) = &edge.label {
            out.push('\n');
            out.push_str(label);
        }
    }

    if let Some(script) = &doc.script {
        for message in script.messages() {
            out.push('\n');
            out.push_str(&message.text);
        }
    }

    out
}

/// Importa y exige que el texto dificil salga tal cual.
fn entra(nombre: &str, language: SourceLanguage, source: &str, esperado: &[&str]) -> Document {
    let result = parse::import(language, source, 0);
    assert!(
        result.ok(),
        "{nombre}: no deberia fallar: {:#?}",
        result.diagnostics
    );

    let todo = texto(&result.document);
    for palabra in esperado {
        assert!(
            todo.contains(palabra),
            "{nombre}: falta `{palabra}`.\nLo que hay:\n{todo}"
        );
    }

    // La senal de que algo se leyo byte a byte: la `Ã` que aparece cuando
    // UTF-8 se interpreta como Latin-1.
    assert!(
        !todo.contains('Ã') && !todo.contains('\u{fffd}'),
        "{nombre}: el texto llego estropeado.\n{todo}"
    );

    result.document
}

/* --- Cada importador por su cuenta -------------------------------------- */

#[test]
fn dbml() {
    entra(
        "dbml",
        SourceLanguage::Dbml,
        "Table artículos {\n  código integer [pk]\n  año integer\n  descripción varchar\n}",
        &["artículos", "código", "año", "descripción"],
    );
}

#[test]
fn mermaid_flowchart() {
    entra(
        "flowchart",
        SourceLanguage::Mermaid,
        "flowchart TD\n\
         \x20 Gestión --> b[Añadir artículo]\n\
         \x20 b -->|validación| c[Año ✅]\n\
         \x20 subgraph Sección\n\
         \x20   d[Coste €]\n\
         \x20 end",
        &[
            "Gestión",
            "Añadir",
            "artículo",
            "validación",
            "Año",
            "✅",
            "€",
            "Sección",
        ],
    );
}

#[test]
fn mermaid_class_diagram() {
    entra(
        "classDiagram",
        SourceLanguage::Mermaid,
        "classDiagram\n\
         \x20 class Artículo {\n\
         \x20   +int código\n\
         \x20   +validación() bool\n\
         \x20 }",
        &["Artículo", "código", "validación"],
    );
}

#[test]
fn mermaid_state_diagram() {
    entra(
        "stateDiagram-v2",
        SourceLanguage::Mermaid,
        "stateDiagram-v2\n  [*] --> Después\n  Después --> Envío : confirmación",
        &["Después", "Envío", "confirmación"],
    );
}

#[test]
fn mermaid_sequence_diagram() {
    entra(
        "sequenceDiagram",
        SourceLanguage::Mermaid,
        "sequenceDiagram\n\
         \x20 participant Navegación\n\
         \x20 participant Validación\n\
         \x20 Navegación->>Validación: petición de año",
        &["Navegación", "Validación", "petición", "año"],
    );
}

#[test]
fn mermaid_er_diagram() {
    entra(
        "erDiagram",
        SourceLanguage::Mermaid,
        "erDiagram\n\
         \x20 CLIENTE ||--o{ PEDIDO : coloca\n\
         \x20 PEDIDO {\n\
         \x20   int código PK\n\
         \x20   string descripción\n\
         \x20 }",
        &["código", "descripción"],
    );
}

#[test]
fn plantuml_componentes() {
    entra(
        "componentes",
        SourceLanguage::PlantUml,
        "@startuml\n[Gestión] --> [Añadir artículo] : petición\n@enduml",
        &["Gestión", "Añadir", "artículo", "petición"],
    );
}

#[test]
fn plantuml_clases() {
    entra(
        "clases",
        SourceLanguage::PlantUml,
        "@startuml\nclass Artículo {\n  +código : int\n  +validación()\n}\n@enduml",
        &["Artículo", "código", "validación"],
    );
}

#[test]
fn plantuml_estados() {
    entra(
        "estados",
        SourceLanguage::PlantUml,
        "@startuml\n[*] --> Después\nDespués --> Envío : confirmación\n@enduml",
        &["Después", "Envío", "confirmación"],
    );
}

#[test]
fn plantuml_secuencia() {
    entra(
        "secuencia",
        SourceLanguage::PlantUml,
        "@startuml\nparticipant Navegación\nparticipant Validación\nNavegación -> Validación : petición\n@enduml",
        &["Navegación", "Validación", "petición"],
    );
}

#[test]
fn plantuml_casos_de_uso() {
    entra(
        "casos de uso",
        SourceLanguage::PlantUml,
        "@startuml\n:Administración: as a\n(Añadir artículo) as u\na --> u\n@enduml",
        &["Administración", "Añadir", "artículo"],
    );
}

#[test]
fn plantuml_actividad() {
    entra(
        "actividad",
        SourceLanguage::PlantUml,
        "@startuml\n\
         start\n\
         :Recibir petición;\n\
         if (¿hay artículos?) then (sí)\n\
         \x20 :Añadir al envío;\n\
         endif\n\
         stop\n\
         @enduml",
        &["petición", "artículos", "Añadir", "envío"],
    );
}

/* --- Y de vuelta ---------------------------------------------------------- */

#[test]
fn el_texto_sobrevive_la_ida_y_vuelta_por_los_tres_idiomas() {
    // Generar el codigo y volver a leerlo es el otro sitio donde se podria
    // romper, y no lo cubre ninguna prueba de las de arriba.
    let casos: &[(&str, SourceLanguage, &str, &[&str])] = &[
        (
            "dbml",
            SourceLanguage::Dbml,
            "Table artículos {\n  código integer [pk]\n  año integer\n}",
            &["artículos", "código", "año"],
        ),
        (
            "flowchart",
            SourceLanguage::Mermaid,
            "flowchart TD\n  Gestión --> b[Añadir artículo]",
            &["Gestión", "Añadir", "artículo"],
        ),
        (
            "componentes",
            SourceLanguage::PlantUml,
            "@startuml\n[Gestión] --> [Añadir artículo] : petición\n@enduml",
            &["Gestión", "Añadir", "artículo", "petición"],
        ),
    ];

    for (nombre, language, source, esperado) in casos {
        let doc = entra(nombre, *language, source, esperado);

        let codigo = emit::emit(*language, &doc);
        for palabra in *esperado {
            assert!(
                codigo.contains(palabra),
                "{nombre}: el codigo generado perdio `{palabra}`:\n{codigo}"
            );
        }

        entra(&format!("{nombre} de vuelta"), *language, &codigo, esperado);
    }
}

#[test]
fn el_texto_sobrevive_al_archivo() {
    let doc = entra(
        "flowchart",
        SourceLanguage::Mermaid,
        "flowchart TD\n  Gestión --> b[Añadir artículo ✅]",
        &["Gestión", "Añadir", "artículo", "✅"],
    );

    let dir = std::env::temp_dir().join(format!("diagrammaker-acentos-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ruta = dir.join("acentos.dgm");

    io::save(&ruta, &doc).expect("deberia guardar");
    let vuelta = io::load(&ruta).expect("deberia abrir");

    assert_eq!(texto(&vuelta), texto(&doc));

    let _ = std::fs::remove_dir_all(&dir);
}

/* --- Y los simbolos que el SVG tiene que escapar --------------------------- */

#[test]
fn un_ampersand_en_una_etiqueta_no_rompe_el_dibujo() {
    // `&` es el principio de una entidad en XML: sin escapar, el SVG no se
    // puede analizar y el PNG sale en blanco.
    let doc = entra(
        "flowchart",
        SourceLanguage::Mermaid,
        "flowchart LR\n  a[Ventas & Marketing] --> b[I+D]",
        &["Ventas & Marketing"],
    );

    assert_eq!(doc.nodes.len(), 2, "el `&` de la etiqueta no parte el nodo");
}
