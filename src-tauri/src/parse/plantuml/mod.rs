//! Importadores de PlantUML.
//!
//! A diferencia de Mermaid, PlantUML no declara de que diagrama se trata:
//! todos empiezan por `@startuml`. Hay que deducirlo del contenido, y este
//! modulo es quien lo hace antes de pasar el texto a quien sepa leerlo.
//!
//! La regla es deliberadamente decisiva, no estadistica:
//!
//! 1. Si aparece algo que **solo** existe en una secuencia —`participant`,
//!    `activate`, un `alt`— es una secuencia.
//! 2. Si aparece algo que solo existe en un diagrama de componentes —un
//!    `[Nombre]`, un bloque `package { }`— es de componentes.
//! 3. Si hay mensajes con etiqueta entre nombres sin declarar
//!    (`Alice -> Bob : hola`), es una secuencia: ese es el primer ejemplo de
//!    la documentacion de PlantUML y seria el peor sitio donde equivocarse.
//! 4. En cualquier otro caso, componentes.
//!
//! El unico caso que queda ambiguo es `A --> B` a secas, sin etiqueta ni
//! declaraciones: ahi las dos lecturas son defendibles y se elige la de
//! componentes, que es la que ya habia.

pub(crate) mod arrows;
pub(crate) mod component;
pub(crate) mod elements;
pub(crate) mod sequence;

use super::ImportResult;

/// Dialectos de PlantUML que se entienden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Component,
    Sequence,
}

/// Palabras que en PlantUML solo tienen sentido dentro de una secuencia.
const SEQUENCE_ONLY: &[&str] = &[
    "participant",
    "activate",
    "deactivate",
    "autonumber",
    "alt",
    "opt",
    "loop",
    "par",
    "critical",
    "break",
    "ref",
    "return",
];

/// Palabras que solo declaran elementos de un diagrama de componentes.
///
/// `actor`, `database` o `queue` quedan fuera a proposito: una secuencia
/// tambien los usa y no distinguirian nada.
const COMPONENT_ONLY: &[&str] = &[
    "component",
    "rectangle",
    "package",
    "node",
    "cloud",
    "folder",
    "frame",
    "card",
    "usecase",
    "storage",
    "artifact",
    "agent",
    "together",
];

pub fn flavor_of(source: &str) -> Flavor {
    let mut looks_like_component = false;
    let mut labelled_messages = false;

    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('\'') || line.starts_with('@') {
            continue;
        }

        let lower = line.to_lowercase();
        let first = lower.split_whitespace().next().unwrap_or("");

        if SEQUENCE_ONLY.contains(&first) {
            return Flavor::Sequence;
        }

        // Un `[Nombre]` o una declaracion con palabra propia solo salen en
        // componentes. El corchete tiene que abrir la linea: `-[#red]->` es
        // una flecha.
        if line.starts_with('[') || COMPONENT_ONLY.contains(&first) {
            looks_like_component = true;
        }

        if let Some(head) = head_of(line) {
            // Una cardinalidad pegada a la flecha (`api "1" --> "n" base`)
            // es sintaxis de componentes: una secuencia no las tiene.
            if has_multiplicity(head) {
                looks_like_component = true;
            } else if sequence::find_arrow(head).is_some() {
                labelled_messages = true;
            }
        }
    }

    match (looks_like_component, labelled_messages) {
        (false, true) => Flavor::Sequence,
        _ => Flavor::Component,
    }
}

/// Lo que va delante de la etiqueta, si la linea lleva una.
fn head_of(line: &str) -> Option<&str> {
    line.contains(" : ")
        .then(|| line.split(" : ").next().unwrap_or(line))
}

/// `true` si alguno de los extremos lleva cardinalidad entrecomillada.
fn has_multiplicity(head: &str) -> bool {
    let Some((at, arrow, ..)) = sequence::find_arrow(head) else {
        return false;
    };

    component::trailing_multiplicity(&head[..at]).1.is_some()
        || component::leading_multiplicity(&head[at + arrow.len()..])
            .1
            .is_some()
}

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    match flavor_of(source) {
        Flavor::Component => component::import(source, now_ms),
        Flavor::Sequence => sequence::import(source, now_ms),
    }
}

pub use arrows::arrow_for;
pub use elements::keyword_for;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::DiagramKind;

    #[test]
    fn una_palabra_de_secuencia_decide_sola() {
        for source in [
            "@startuml\nparticipant A\n@enduml",
            "@startuml\nA -> B\nactivate B\n@enduml",
            "@startuml\nalt si\nend\n@enduml",
            "@startuml\nautonumber\nA -> B\n@enduml",
        ] {
            assert_eq!(flavor_of(source), Flavor::Sequence, "{source}");
        }
    }

    #[test]
    fn los_corchetes_y_los_bloques_delatan_un_diagrama_de_componentes() {
        for source in [
            "@startuml\n[Portal] --> [API]\n@enduml",
            "@startuml\npackage \"Capa\" {\n  component X\n}\n@enduml",
        ] {
            assert_eq!(flavor_of(source), Flavor::Component, "{source}");
        }
    }

    #[test]
    fn el_ejemplo_canonico_de_plantuml_es_una_secuencia() {
        // `Alice -> Bob : hola` sin declarar nada es el primer ejemplo de la
        // documentacion de PlantUML; leerlo como componentes seria el peor
        // sitio donde equivocarse.
        assert_eq!(
            flavor_of("@startuml\nAlice -> Bob : Hola\nBob --> Alice : Que tal\n@enduml"),
            Flavor::Sequence
        );
    }

    #[test]
    fn una_etiqueta_entre_componentes_declarados_no_enganna() {
        let source = "@startuml\n[Portal] --> [API] : HTTPS\n@enduml";

        assert_eq!(flavor_of(source), Flavor::Component);
    }

    #[test]
    fn sin_pistas_se_lee_como_componentes() {
        // Las dos lecturas son defendibles; se elige la que ya habia.
        assert_eq!(flavor_of("@startuml\nA --> B\n@enduml"), Flavor::Component);
        assert_eq!(flavor_of("@startuml\n@enduml"), Flavor::Component);
    }

    #[test]
    fn los_comentarios_no_despistan() {
        let source = "@startuml\n' participant A\n[X] --> [Y]\n@enduml";

        assert_eq!(flavor_of(source), Flavor::Component);
    }

    #[test]
    fn el_despachador_lleva_cada_uno_a_su_sitio() {
        assert_eq!(
            import("@startuml\nparticipant A\nA -> A : x\n@enduml", 0)
                .document
                .kind,
            DiagramKind::Sequence
        );
        assert_eq!(
            import("@startuml\n[A] --> [B]\n@enduml", 0).document.kind,
            DiagramKind::Architecture
        );
    }
}
