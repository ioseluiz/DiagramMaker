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
