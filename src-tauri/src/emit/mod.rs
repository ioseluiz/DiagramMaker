//! Generadores de codigo a partir del documento.
//!
//! Son la vuelta del camino que abren los importadores de `parse`: lo que sale
//! de aqui tiene que poder volver a entrar por alli y producir el mismo
//! diagrama. Las pruebas de ida y vuelta de cada modulo son las que lo fijan.

pub mod dbml;
pub mod mermaid;
pub mod plantuml;

use crate::model::Document;
use crate::parse::SourceLanguage;

/// Genera el codigo del documento en el lenguaje indicado.
pub fn emit(language: SourceLanguage, doc: &Document) -> String {
    match language {
        SourceLanguage::Dbml => dbml::emit(doc),
        SourceLanguage::Mermaid => mermaid::emit(doc),
        SourceLanguage::PlantUml => plantuml::emit(doc),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{DiagramKind, SequenceScript};

    /// Entra por la puerta publica, no por cada submodulo.
    ///
    /// Es la prueba que faltaba: los generadores de cada dialecto se probaban
    /// llamandolos directamente, asi que el despachador de PlantUML se quedo
    /// sin el `if` de la secuencia y nadie se entero. Una secuencia salia por
    /// el camino de los componentes, con los participantes como cajas sueltas
    /// y sin un solo mensaje, porque una secuencia no tiene aristas.
    fn header_of(language: SourceLanguage, kind: DiagramKind) -> String {
        let mut doc = Document::new("d", kind, 0);
        if kind.is_lane_based() {
            doc.script = Some(SequenceScript::default());
        }

        emit(language, &doc)
            .lines()
            .next()
            .unwrap_or("")
            .to_string()
    }

    #[test]
    fn cada_tipo_de_diagrama_sale_por_el_dialecto_que_le_toca_en_mermaid() {
        assert_eq!(
            header_of(SourceLanguage::Mermaid, DiagramKind::Architecture),
            "flowchart LR"
        );
        assert_eq!(
            header_of(SourceLanguage::Mermaid, DiagramKind::Sequence),
            "sequenceDiagram"
        );
        assert_eq!(
            header_of(SourceLanguage::Mermaid, DiagramKind::Class),
            "classDiagram"
        );
    }

    #[test]
    fn y_lo_mismo_en_plantuml() {
        // Los tres empiezan por `@startuml`, asi que la cabecera no basta:
        // lo que distingue a cada uno es la segunda linea.
        let body = |kind: DiagramKind| {
            let mut doc = Document::new("d", kind, 0);
            if kind.is_lane_based() {
                doc.script = Some(SequenceScript::default());
            }
            emit(SourceLanguage::PlantUml, &doc)
        };

        // Un diagrama de componentes declara su direccion de layout; los otros
        // dos no tienen esa linea.
        assert!(
            body(DiagramKind::Architecture).contains("left to right direction"),
            "{}",
            body(DiagramKind::Architecture)
        );
        assert!(!body(DiagramKind::Sequence).contains("direction"));
        assert!(!body(DiagramKind::Class).contains("direction"));
    }

    #[test]
    fn una_secuencia_no_sale_por_el_camino_de_los_componentes() {
        use crate::model::{Message, MessageKind, Node, NodeShape, Step};

        let mut doc = Document::new("d", DiagramKind::Sequence, 0);
        doc.nodes
            .push(Node::new("a", "Cliente", NodeShape::Rectangle));
        doc.nodes.push(Node::new("b", "API", NodeShape::Rectangle));
        doc.script = Some(SequenceScript {
            steps: vec![Step::Message(
                Message::new("m1", "a", "b", MessageKind::Sync).with_text("comprar"),
            )],
        });

        for language in [SourceLanguage::Mermaid, SourceLanguage::PlantUml] {
            let code = emit(language, &doc);
            assert!(
                code.contains("comprar"),
                "{language:?} perdio el mensaje:\n{code}"
            );
        }
    }

    #[test]
    fn un_diagrama_de_clases_no_sale_por_el_camino_de_los_componentes() {
        use crate::model::{ClassBox, ClassMember, Node, NodeData, NodeShape};

        let mut doc = Document::new("d", DiagramKind::Class, 0);
        let mut node = Node::new("k1", "Pedido", NodeShape::Class);
        node.data = NodeData::Class(
            ClassBox::new().with_attributes(vec![ClassMember::new("a1", "id").with_type("int")]),
        );
        doc.nodes.push(node);

        for language in [SourceLanguage::Mermaid, SourceLanguage::PlantUml] {
            let code = emit(language, &doc);
            // Se busca el tipo y no el nombre: `id` esta dentro de `Pedido`,
            // asi que un generador que solo escribiera la caja lo colaria.
            assert!(
                code.contains("int"),
                "{language:?} perdio el miembro:\n{code}"
            );
        }
    }
}
