//! Guion de un diagrama de secuencia.
//!
//! Es el unico tipo de diagrama que no guarda geometria. Un diagrama de
//! secuencia *es* un orden: quien habla con quien y en que momento. Las
//! coordenadas salen enteras de ese orden, asi que guardarlas seria guardar
//! dos veces la misma informacion y arriesgarse a que no coincidan.
//!
//! Los participantes no viven aqui: son los `nodes` del documento, en su
//! orden, y su forma dice de que tipo son. Asi el inspector, el deshacer y la
//! exportacion siguen funcionando sin saber nada de secuencias. Lo unico que
//! el guion anade es la lista ordenada de pasos.
//!
//! Los pasos anidan —un `alt` contiene mensajes, y puede contener otro `alt`—
//! asi que `Step` es un arbol, no una lista plana.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Lo que pasa en el diagrama, en orden.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct SequenceScript {
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum Step {
    Message(Message),
    Block(Block),
    Note(SequenceNote),
}

impl Step {
    pub fn id(&self) -> &str {
        match self {
            Step::Message(m) => &m.id,
            Step::Block(b) => &b.id,
            Step::Note(n) => &n.id,
        }
    }
}

/// Una flecha entre dos lineas de vida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Message {
    pub id: String,
    /// Id del nodo que envia.
    pub from: String,
    /// Id del nodo que recibe. Igual que `from` en un mensaje a si mismo.
    pub to: String,
    pub kind: MessageKind,

    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,

    /// Abre una barra de ejecucion en el destino.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub activate: bool,
    /// Cierra la barra de ejecucion del origen.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub deactivate: bool,
}

impl Message {
    pub fn new(
        id: impl Into<String>,
        from: impl Into<String>,
        to: impl Into<String>,
        kind: MessageKind,
    ) -> Self {
        Message {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            kind,
            text: String::new(),
            activate: false,
            deactivate: false,
        }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    pub fn activating(mut self) -> Self {
        self.activate = true;
        self
    }

    pub fn deactivating(mut self) -> Self {
        self.deactivate = true;
        self
    }

    /// Un mensaje a uno mismo se dibuja distinto: no cruza, vuelve.
    pub fn is_self(&self) -> bool {
        self.from == self.to
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum MessageKind {
    /// Llamada sincrona: linea solida, punta rellena.
    Sync,
    /// Llamada asincrona: linea solida, punta abierta.
    Async,
    /// Retorno: linea punteada, punta abierta.
    Reply,
    /// Crea al destinatario; su linea de vida empieza aqui.
    Create,
    /// Destruye al destinatario; su linea de vida acaba en una cruz.
    Destroy,
}

impl MessageKind {
    pub fn is_dashed(self) -> bool {
        matches!(self, MessageKind::Reply | MessageKind::Create)
    }

    /// `true` si la punta va rellena.
    pub fn is_filled(self) -> bool {
        matches!(self, MessageKind::Sync | MessageKind::Destroy)
    }
}

/// Un marco que agrupa pasos: `alt`, `loop`, `par`...
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Block {
    pub id: String,
    pub kind: BlockKind,
    /// Siempre al menos una. Solo `alt` y `par` suelen llevar varias.
    pub branches: Vec<Branch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum BlockKind {
    Alt,
    Opt,
    Loop,
    Par,
    Critical,
    Break,
    Group,
}

impl BlockKind {
    /// Etiqueta de la esquina del marco.
    pub fn tag(self) -> &'static str {
        match self {
            BlockKind::Alt => "alt",
            BlockKind::Opt => "opt",
            BlockKind::Loop => "loop",
            BlockKind::Par => "par",
            BlockKind::Critical => "critical",
            BlockKind::Break => "break",
            BlockKind::Group => "group",
        }
    }

    /// Como se llama la rama que no es la primera.
    ///
    /// En un `alt` es un `else`; en un `par`, otro hilo que corre a la vez.
    pub fn divider_tag(self) -> &'static str {
        match self {
            BlockKind::Alt => "else",
            BlockKind::Par => "and",
            _ => "else",
        }
    }
}

/// Una rama de un marco: su condicion y lo que pasa dentro.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Branch {
    pub id: String,
    /// La condicion, o el numero de vueltas de un `loop`. Puede ir vacia.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default)]
    pub steps: Vec<Step>,
}

impl Branch {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Branch {
            id: id.into(),
            label: label.into(),
            steps: Vec::new(),
        }
    }

    pub fn with_steps(mut self, steps: Vec<Step>) -> Self {
        self.steps = steps;
        self
    }
}

/// Una nota pegada a una o varias lineas de vida.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct SequenceNote {
    pub id: String,
    pub placement: NotePlacement,
    /// Nodos a los que se pega. Uno, o los dos extremos de un `over`.
    pub participants: Vec<String>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum NotePlacement {
    LeftOf,
    RightOf,
    Over,
}

impl SequenceScript {
    /// Recorre todos los pasos, entrando en los marcos, en orden de lectura.
    pub fn walk(&self) -> Vec<&Step> {
        let mut out = Vec::new();
        collect(&self.steps, &mut out);
        out
    }

    /// Mensajes del guion, en orden.
    pub fn messages(&self) -> Vec<&Message> {
        self.walk()
            .into_iter()
            .filter_map(|step| match step {
                Step::Message(message) => Some(message),
                _ => None,
            })
            .collect()
    }
}

fn collect<'a>(steps: &'a [Step], out: &mut Vec<&'a Step>) {
    for step in steps {
        out.push(step);
        if let Step::Block(block) = step {
            for branch in &block.branches {
                collect(&branch.steps, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(id: &str) -> Step {
        Step::Message(Message::new(id, "a", "b", MessageKind::Sync))
    }

    #[test]
    fn un_mensaje_a_uno_mismo_se_reconoce() {
        let solo = Message::new("m", "a", "a", MessageKind::Sync);
        let cruzado = Message::new("m", "a", "b", MessageKind::Sync);

        assert!(solo.is_self());
        assert!(!cruzado.is_self());
    }

    #[test]
    fn el_retorno_va_punteado_y_la_llamada_no() {
        assert!(MessageKind::Reply.is_dashed());
        assert!(!MessageKind::Sync.is_dashed());
        assert!(MessageKind::Sync.is_filled());
        assert!(!MessageKind::Async.is_filled());
    }

    #[test]
    fn el_recorrido_entra_en_los_marcos() {
        let script = SequenceScript {
            steps: vec![
                message("m1"),
                Step::Block(Block {
                    id: "b1".into(),
                    kind: BlockKind::Alt,
                    branches: vec![
                        Branch::new("r1", "ok").with_steps(vec![message("m2")]),
                        Branch::new("r2", "error").with_steps(vec![message("m3")]),
                    ],
                }),
                message("m4"),
            ],
        };

        let ids: Vec<_> = script.walk().iter().map(|s| s.id()).collect();
        assert_eq!(ids, ["m1", "b1", "m2", "m3", "m4"]);

        let mensajes: Vec<_> = script.messages().iter().map(|m| m.id.as_str()).collect();
        assert_eq!(mensajes, ["m1", "m2", "m3", "m4"]);
    }

    #[test]
    fn los_marcos_anidan() {
        let script = SequenceScript {
            steps: vec![Step::Block(Block {
                id: "fuera".into(),
                kind: BlockKind::Loop,
                branches: vec![Branch::new("r1", "").with_steps(vec![Step::Block(Block {
                    id: "dentro".into(),
                    kind: BlockKind::Opt,
                    branches: vec![
                        Branch::new("r2", "si hay stock").with_steps(vec![message("m")]),
                    ],
                })])],
            })],
        };

        let ids: Vec<_> = script.walk().iter().map(|s| s.id()).collect();
        assert_eq!(ids, ["fuera", "dentro", "m"]);
    }

    #[test]
    fn un_guion_vacio_no_tiene_pasos() {
        assert!(SequenceScript::default().walk().is_empty());
    }
}
