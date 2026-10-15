use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Posicion dentro del codigo fuente de una importacion. Ambas son 1-based,
/// que es como las cuenta un editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Span {
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub fn new(line: u32, column: u32) -> Self {
        Span { line, column }
    }
}

/// Un problema detectado, ya sea al validar el documento o al importar codigo.
///
/// La misma estructura sirve a los dos casos: la validacion rellena `target`
/// (el nodo o arista culpable) y los importadores rellenan `span` (la linea de
/// codigo culpable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Diagnostic {
    pub severity: Severity,

    /// Codigo estable y legible por maquina, p. ej. `duplicateId`. El frontend
    /// puede enlazarlo a una ayuda sin depender del texto del mensaje.
    pub code: String,

    pub message: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<Span>,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            target: None,
            span: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            ..Diagnostic::error(code, message)
        }
    }

    pub fn at(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn at_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// Resultado de validar. `ok` es falso en cuanto hay un solo diagnostico de
/// severidad `Error`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Report {
    pub ok: bool,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        let ok = !diagnostics.iter().any(|d| d.severity == Severity::Error);
        Report { ok, diagnostics }
    }

    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_informe_sin_diagnosticos_esta_ok() {
        assert!(Report::new(vec![]).ok);
    }

    #[test]
    fn las_advertencias_no_tumban_el_informe() {
        let r = Report::new(vec![Diagnostic::warning("w", "cuidado")]);

        assert!(r.ok);
        assert_eq!(r.errors().count(), 0);
    }

    #[test]
    fn un_solo_error_tumba_el_informe() {
        let r = Report::new(vec![
            Diagnostic::warning("w", "cuidado"),
            Diagnostic::error("e", "roto").at("n1"),
        ]);

        assert!(!r.ok);
        assert_eq!(r.errors().count(), 1);
        assert_eq!(r.errors().next().unwrap().target.as_deref(), Some("n1"));
    }
}
