//! Importadores de codigo a documento.
//!
//! Todos siguen la misma regla: producen un `Document` **sin geometria** y una
//! lista de diagnosticos con linea y columna. Colocar los nodos es cosa del
//! auto-layout del frontend, que es quien puede medir el texto.
//!
//! Lo que no se entiende nunca se interpreta a ojo: se reporta con su
//! posicion exacta y se sigue leyendo el resto. Un `Ref` mal escrito no debe
//! costar las veinte tablas que vienen detras.

pub mod dbml;
pub mod mermaid;
pub mod plantuml;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::{Diagnostic, Document};

/// Resultado de importar: el documento y lo que haya que contarle al usuario.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct ImportResult {
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

impl ImportResult {
    pub fn new(document: Document, diagnostics: Vec<Diagnostic>) -> Self {
        ImportResult {
            document,
            diagnostics,
        }
    }

    /// `true` si no hubo ningun diagnostico de severidad `Error`.
    pub fn ok(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|d| d.severity == crate::model::Severity::Error)
    }
}

/// Lenguajes de entrada soportados.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum SourceLanguage {
    /// Sintaxis de dbdiagram.io.
    Dbml,
    /// `flowchart`, `sequenceDiagram`, `classDiagram` y `stateDiagram` de
    /// Mermaid.
    Mermaid,
    /// Diagramas de componentes, secuencia, clases y estados de PlantUML.
    PlantUml,
}

impl SourceLanguage {
    pub fn label(self) -> &'static str {
        match self {
            SourceLanguage::Dbml => "DBML (dbdiagram.io)",
            SourceLanguage::Mermaid => "Mermaid (flowchart, secuencia, clases y estados)",
            SourceLanguage::PlantUml => "PlantUML (componentes, secuencia, clases y estados)",
        }
    }
}

/// Importa `source` en el lenguaje indicado.
pub fn import(language: SourceLanguage, source: &str, now_ms: i64) -> ImportResult {
    match language {
        SourceLanguage::Dbml => dbml::import(source, now_ms),
        SourceLanguage::Mermaid => mermaid::import(source, now_ms),
        SourceLanguage::PlantUml => plantuml::import(source, now_ms),
    }
}
