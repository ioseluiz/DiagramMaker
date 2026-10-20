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
//! 2. Si aparece algo que solo existe en un diagrama de clases —`class`,
//!    `abstract`, `enum`— es de clases.
//! 3. Si aparece `state` o un `[*]`, es una maquina de estados: ese corchete
//!    no existe en ningun otro dialecto.
//! 4. Si aparece un `usecase` o una elipse `(Caso de uso)`, es de casos de
//!    uso: ese parentesis no abre ninguna linea de los demas dialectos.
//! 5. Si aparece algo que solo existe en un diagrama de componentes —un
//!    `[Nombre]`, un bloque `package { }`— es de componentes.
//! 6. Si hay mensajes con etiqueta entre nombres sin declarar
//!    (`Alice -> Bob : hola`), es una secuencia: ese es el primer ejemplo de
//!    la documentacion de PlantUML y seria el peor sitio donde equivocarse.
//! 7. En cualquier otro caso, componentes.
//!
//! Quedan dos casos ambiguos, y en los dos se elige la lectura de
//! componentes, que es la que ya habia: `A --> B` a secas, sin etiqueta ni
//! declaraciones, y un `@startuml` que solo declare `interface`, palabra que
//! los dos dialectos usan.

pub(crate) mod arrows;
pub(crate) mod class_diagram;
pub(crate) mod component;
pub(crate) mod elements;
pub(crate) mod sequence;
pub(crate) mod state_diagram;

use super::ImportResult;

/// Dialectos de PlantUML que se entienden.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavor {
    Component,
    UseCase,
    Sequence,
    Class,
    State,
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

/// Palabras que en PlantUML solo tienen sentido en un diagrama de clases.
///
/// `interface` queda fuera a proposito: un diagrama de componentes tambien la
/// usa, y ahi dibuja un puerto. Un `@startuml` que solo declare interfaces se
/// sigue leyendo como de componentes, que es lo que ya habia.
const CLASS_ONLY: &[&str] = &[
    "class",
    "abstract",
    "enum",
    "annotation",
    "struct",
    "protocol",
    "exception",
    "metaclass",
];

/// Lo que escribe PlantUML donde empieza o acaba una maquina de estados.
const TERMINAL: &str = "[*]";

/// Palabras que en PlantUML solo tienen sentido en una maquina de estados.
const STATE_ONLY: &[&str] = &["state"];

/// Palabras que en PlantUML solo tienen sentido en un diagrama de casos de uso.
///
/// `actor` queda fuera a proposito: una secuencia y un diagrama de componentes
/// tambien lo usan, asi que no distinguiria nada.
const USECASE_ONLY: &[&str] = &["usecase"];

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
        if CLASS_ONLY.contains(&first) {
            return Flavor::Class;
        }
        // `[*]` es el arranque o la salida de una maquina de estados, y no
        // existe en ningun otro dialecto.
        if STATE_ONLY.contains(&first) || line.contains(TERMINAL) {
            return Flavor::State;
        }
        // Una elipse tiene que abrir la linea para contar: en `A --> (B)` el
        // parentesis tambien dice que `B` es un caso de uso, pero ahi hay un
        // nombre suelto delante que podria ser cualquier cosa.
        if USECASE_ONLY.contains(&first) || line.starts_with('(') {
            return Flavor::UseCase;
        }

        // Un `[Nombre]` o una declaracion con palabra propia solo salen en
        // componentes. El corchete tiene que abrir la linea: `-[#red]->` es
        // una flecha.
        if (line.starts_with('[') && !line.starts_with(TERMINAL)) || COMPONENT_ONLY.contains(&first)
        {
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
        Flavor::UseCase => component::import_as(source, now_ms, crate::model::DiagramKind::UseCase),
        Flavor::Sequence => sequence::import(source, now_ms),
        Flavor::Class => class_diagram::import(source, now_ms),
        Flavor::State => state_diagram::import(source, now_ms),
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
    fn una_palabra_de_clases_decide_sola() {
        for source in [
            "@startuml\nclass Pedido\n@enduml",
            "@startuml\nabstract class Base\n@enduml",
            "@startuml\nenum Estado {\n  NUEVO\n}\n@enduml",
        ] {
            assert_eq!(flavor_of(source), Flavor::Class, "{source}");
        }
    }

    #[test]
    fn una_palabra_de_estados_o_un_corchete_asterisco_deciden_solos() {
        for source in [
            "@startuml\nstate Nuevo\n@enduml",
            "@startuml\n[*] --> Nuevo\n@enduml",
            "@startuml\nNuevo --> [*]\n@enduml",
        ] {
            assert_eq!(flavor_of(source), Flavor::State, "{source}");
        }
    }

    #[test]
    fn un_usecase_o_una_elipse_deciden_solos() {
        for source in [
            "@startuml
usecase \"Comprar\" as uc1
@enduml",
            "@startuml
(Comprar)
@enduml",
            "@startuml
left to right direction
actor Cliente
(Comprar)
@enduml",
        ] {
            assert_eq!(flavor_of(source), Flavor::UseCase, "{source}");
        }
    }

    #[test]
    fn un_corchete_de_componente_sigue_siendo_de_componentes() {
        // `[Portal]` y `[*]` empiezan los dos por corchete; solo el segundo
        // es una maquina de estados.
        assert_eq!(
            flavor_of("@startuml\n[Portal] --> [API]\n@enduml"),
            Flavor::Component
        );
    }

    #[test]
    fn una_interfaz_sola_sigue_siendo_de_componentes() {
        // `interface` existe en los dos dialectos; sin mas pistas se queda
        // con la lectura que ya habia.
        assert_eq!(
            flavor_of("@startuml\ninterface HTTP\n@enduml"),
            Flavor::Component
        );
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
        assert_eq!(
            import("@startuml\n(Comprar)\n@enduml", 0).document.kind,
            DiagramKind::UseCase
        );
    }
}
