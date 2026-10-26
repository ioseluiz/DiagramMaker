//! Importador de `erDiagram` de Mermaid.
//!
//! Comparte modelo con DBML —son el mismo diagrama escrito de otra manera—
//! pero tiene tres diferencias que marcan como se lee:
//!
//! - **La relacion va entre entidades, no entre columnas.** DBML escribe
//!   `Ref: pedidos.cliente_id > clientes.id`; Mermaid escribe
//!   `CLIENTE ||--o{ PEDIDO : coloca`. Las aristas llegan sin puerto, y es el
//!   atributo quien dice cual es la clave ajena con un `FK` explicito.
//! - **El tipo va delante del nombre** (`string nombre`), al reves que en
//!   DBML.
//! - **La cardinalidad son cuatro simbolos a cada lado**, no dos. Mermaid
//!   distingue "cero o uno" de "exactamente uno" y "cero o varios" de "uno o
//!   varios"; el documento solo guarda `1` y `*`.
//!
//! Esa ultima diferencia pierde informacion, y es a proposito. La
//! cardinalidad de un entidad-relacion es, en toda la aplicacion, una de
//! cuatro combinaciones de `1` y `*`: el inspector la ofrece como lista
//! cerrada y el generador de DBML cuenta con ello. Guardar aqui un `0..*` que
//! el resto del programa no entiende seria peor que redondearlo, asi que lo
//! opcional se redondea a obligatorio y se dice en la documentacion en vez de
//! fingir que cabe.
//!
//! Lo que si sobrevive es si la relacion es identificadora: el `--` y el `..`
//! de Mermaid se guardan como trazo continuo o discontinuo.

use std::collections::HashMap;

use crate::model::{
    Diagnostic, DiagramKind, Document, Edge, EdgeKind, ErColumn, ErTable, LayoutDirection, Node,
    Routing, Span,
};

use super::super::{ImportResult, strip_word};
use super::strip_comment;

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut parser = Parser::new(now_ms);
    parser.run(source);
    parser.finish()
}

/// Simbolos de cardinalidad del lado izquierdo, con lo que guarda el modelo.
///
/// `|o` y `||` se distinguen en Mermaid —"cero o uno" frente a "exactamente
/// uno"— pero aqui los dos son `1`.
const LEFT: &[(&str, &str)] = &[("|o", "1"), ("||", "1"), ("}o", "*"), ("}|", "*")];

/// Los mismos, escritos en espejo porque miran al otro lado.
const RIGHT: &[(&str, &str)] = &[("o|", "1"), ("||", "1"), ("o{", "*"), ("|{", "*")];

/// Lo que separa las dos cardinalidades. El punteado es la relacion no
/// identificadora.
const LINES: &[(&str, bool)] = &[("--", false), ("..", true)];

/// Marcas que puede llevar un atributo detras del nombre.
const KEYS: &[&str] = &["pk", "fk", "uk"];

struct Parser {
    document: Document,
    diagnostics: Vec<Diagnostic>,
    /// Entidad por nombre. Mermaid distingue mayusculas, asi que la clave es
    /// el nombre tal cual.
    by_name: HashMap<String, usize>,
    /// Entidad cuyo bloque de atributos esta abierto.
    open: Option<usize>,
    counter: usize,
}

impl Parser {
    fn new(now_ms: i64) -> Self {
        Parser {
            document: Document::new(uuid::Uuid::new_v4().to_string(), DiagramKind::Er, now_ms),
            diagnostics: Vec::new(),
            by_name: HashMap::new(),
            open: None,
            counter: 0,
        }
    }

    fn finish(mut self) -> ImportResult {
        if self.open.is_some() {
            self.diagnostics.push(Diagnostic::error(
                "unterminatedEntity",
                "Falta la `}` que cierra el bloque de atributos.",
            ));
        }

        ImportResult::new(self.document, self.diagnostics)
    }

    fn id(&mut self, prefix: &str) -> String {
        self.counter += 1;
        format!("{prefix}{}", self.counter)
    }

    fn error(&mut self, code: &str, message: impl Into<String>, line: u32, column: u32) {
        self.diagnostics
            .push(Diagnostic::error(code, message).at_span(Span::new(line, column)));
    }

    fn run(&mut self, source: &str) {
        let mut seen_header = false;

        for (index, raw) in source.lines().enumerate() {
            let line = index as u32 + 1;
            let text = strip_comment(raw).trim_end();
            let column = (text.len() - text.trim_start().len()) as u32 + 1;
            let text = text.trim();

            if text.is_empty() {
                continue;
            }

            if !seen_header {
                seen_header = true;
                if text.eq_ignore_ascii_case("erdiagram") {
                    continue;
                }
            }

            self.line(text, line, column);
        }
    }

    fn line(&mut self, text: &str, line: u32, column: u32) {
        if text == "}" {
            if self.open.take().is_none() {
                self.error(
                    "unexpectedClose",
                    "`}` sin bloque que cerrar.",
                    line,
                    column,
                );
            }
            return;
        }

        // Dentro de un bloque todo son atributos: una relacion escrita ahi es
        // un error del autor, no otra cosa que leer.
        if let Some(index) = self.open {
            self.attribute(index, text, line, column);
            return;
        }

        if let Some(rest) = keyword(text, "title") {
            let title = rest.trim();
            if !title.is_empty() {
                self.document.meta.title = title.to_string();
            }
            return;
        }

        if let Some(rest) = keyword(text, "direction") {
            match direction_of(rest.trim()) {
                Some(direction) => self.document.direction = direction,
                None => self.error(
                    "badDirection",
                    format!("Direccion `{}` desconocida.", rest.trim()),
                    line,
                    column,
                ),
            }
            return;
        }

        if find_relation(text).is_some() {
            self.relation(text, line, column);
            return;
        }

        self.declaration(text, line, column);
    }

    /* --- Entidades --------------------------------------------------------- */

    /// `CLIENTE`, `CLIENTE {`, `PEDIDO["Pedido de venta"] {`.
    fn declaration(&mut self, text: &str, line: u32, column: u32) {
        let opens = text.ends_with('{');
        let head = if opens {
            text[..text.len() - 1].trim_end()
        } else {
            text
        };

        let Some((name, alias, rest)) = read_entity(head) else {
            self.error(
                "badStatement",
                format!("No se entiende `{text}`."),
                line,
                column,
            );
            return;
        };

        if !rest.trim().is_empty() {
            self.error(
                "badStatement",
                format!("Sobra `{}` detras de `{name}`.", rest.trim()),
                line,
                column,
            );
            return;
        }

        let index = self.entity(&name, alias);
        if opens {
            self.open = Some(index);
        }
    }

    /// Indice de la entidad, creandola si es la primera vez que se nombra.
    fn entity(&mut self, name: &str, alias: Option<String>) -> usize {
        if let Some(&index) = self.by_name.get(name) {
            // El alias puede llegar despues de la primera mencion, cuando la
            // entidad ya existia por aparecer en una relacion.
            if let Some(alias) = alias {
                let node = &mut self.document.nodes[index];
                node.label = alias;
                if let Some(table) = node.data.as_er_mut() {
                    table.alias = Some(name.to_string());
                }
            }
            return index;
        }

        let id = self.id("t");
        let (label, stored) = match alias {
            Some(alias) => (alias, Some(name.to_string())),
            None => (name.to_string(), None),
        };

        let table = ErTable {
            alias: stored,
            ..Default::default()
        };

        self.document.nodes.push(Node::table(id, label, table));
        let index = self.document.nodes.len() - 1;
        self.by_name.insert(name.to_string(), index);

        index
    }

    /* --- Atributos ---------------------------------------------------------- */

    /// `string nombre`, `int id PK`, `int cliente_id FK "quien lo pidio"`.
    fn attribute(&mut self, index: usize, text: &str, line: u32, column: u32) {
        // El bloque puede cerrarse en la misma linea del ultimo atributo.
        let (text, closes) = match text.strip_suffix('}') {
            Some(rest) => (rest.trim_end(), true),
            None => (text, false),
        };

        if !text.is_empty() {
            match read_attribute(text) {
                Some(column_) => {
                    let id = self.id("c");
                    let mut parsed = column_;
                    parsed.id = id;
                    if let Some(table) = self.document.nodes[index].data.as_er_mut() {
                        table.columns.push(parsed);
                    }
                }
                None => self.error(
                    "badAttribute",
                    format!("No se entiende el atributo `{text}`. Se escribe `tipo nombre`."),
                    line,
                    column,
                ),
            }
        }

        if closes {
            self.open = None;
        }
    }

    /* --- Relaciones --------------------------------------------------------- */

    fn relation(&mut self, text: &str, line: u32, column: u32) {
        let Some(found) = find_relation(text) else {
            return;
        };

        let Some((from, _, from_rest)) = read_entity(text[..found.at].trim()) else {
            self.error(
                "danglingRelation",
                "Falta la entidad de la izquierda.",
                line,
                column,
            );
            return;
        };
        if !from_rest.trim().is_empty() {
            self.error(
                "badStatement",
                format!("Sobra `{}` delante de la relacion.", from_rest.trim()),
                line,
                column,
            );
            return;
        }

        let tail = &text[found.at + found.width..];
        let (right, label) = split_label(tail);

        let Some((to, _, to_rest)) = read_entity(right.trim()) else {
            self.error(
                "danglingRelation",
                "Falta la entidad de la derecha.",
                line,
                column,
            );
            return;
        };
        if !to_rest.trim().is_empty() {
            self.error(
                "badStatement",
                format!("Sobra `{}` detras de la entidad.", to_rest.trim()),
                line,
                column,
            );
            return;
        }

        let source = self.entity(&from, None);
        let target = self.entity(&to, None);

        let id = self.id("r");
        let mut edge = Edge::new(
            id,
            self.document.nodes[source].id.clone(),
            self.document.nodes[target].id.clone(),
            EdgeKind::Association,
        );
        edge.source.label = Some(found.from.to_string());
        edge.target.label = Some(found.to.to_string());
        edge.routing = Routing::Orthogonal;
        edge.label = label;

        // La relacion no identificadora se dibuja con trazo discontinuo, que
        // es lo que hace Mermaid y lo unico que el modelo sabe guardar de
        // esa distincion.
        if found.dashed {
            edge.style.dashed = Some(true);
        }

        self.document.edges.push(edge);
    }
}

/* --- Lectura de piezas ------------------------------------------------------ */

/// Donde esta el operador de una relacion y que dice.
struct Relation {
    at: usize,
    width: usize,
    from: &'static str,
    to: &'static str,
    dashed: bool,
}

/// Busca el operador: dos simbolos de cardinalidad con una linea en medio.
///
/// Se busca primero la linea (`--` o `..`) y luego se mira si lo que la rodea
/// son cardinalidades. Asi un guion suelto en un nombre no confunde.
fn find_relation(text: &str) -> Option<Relation> {
    let bytes = text.as_bytes();

    for at in 2..bytes.len().saturating_sub(3) {
        if !text.is_char_boundary(at) || !text.is_char_boundary(at + 2) {
            continue;
        }

        let Some(&(_, dashed)) = LINES.iter().find(|(l, _)| text[at..].starts_with(l)) else {
            continue;
        };

        let Some(&(_, from)) = LEFT.iter().find(|(s, _)| &&text[at - 2..at] == s) else {
            continue;
        };

        let after = at + 2;
        if !text.is_char_boundary(after + 2) {
            continue;
        }
        let Some(&(_, to)) = RIGHT.iter().find(|(s, _)| &&text[after..after + 2] == s) else {
            continue;
        };

        return Some(Relation {
            at: at - 2,
            width: 6,
            from,
            to,
            dashed,
        });
    }

    None
}

/// Lee el nombre de una entidad y su alias. Devuelve tambien lo que sobra.
///
/// El nombre puede ir entrecomillado (`"LINEA-DE-PEDIDO"`) y llevar detras un
/// alias entre corchetes (`PEDIDO["Pedido de venta"]`), que es como Mermaid
/// separa el identificador del texto que se dibuja.
fn read_entity(text: &str) -> Option<(String, Option<String>, &str)> {
    let text = text.trim_start();
    if text.is_empty() {
        return None;
    }

    let (name, rest) = match text.strip_prefix('"') {
        Some(inner) => {
            let end = inner.find('"')?;
            (inner[..end].to_string(), &inner[end + 1..])
        }
        None => {
            let end = text
                .find(|c: char| c.is_whitespace() || c == '[' || c == '{')
                .unwrap_or(text.len());
            if end == 0 {
                return None;
            }
            (text[..end].to_string(), &text[end..])
        }
    };

    let Some(inner) = rest.strip_prefix('[') else {
        return Some((name, None, rest));
    };

    let end = inner.find(']')?;
    let alias = inner[..end].trim().trim_matches('"').to_string();

    Some((name, Some(alias), &inner[end + 1..]))
}

/// Parte la etiqueta de la relacion, que va detras de dos puntos.
fn split_label(text: &str) -> (&str, Option<String>) {
    let Some((head, label)) = text.split_once(':') else {
        return (text, None);
    };

    let label = label.trim().trim_matches('"').trim();

    (head, (!label.is_empty()).then(|| label.to_string()))
}

/// Lee `tipo nombre [PK, FK] ["comentario"]`.
fn read_attribute(text: &str) -> Option<ErColumn> {
    // El comentario va al final entre comillas; se quita antes de separar en
    // palabras para que sus espacios no cuenten.
    let (head, note) = match text.strip_suffix('"') {
        Some(rest) => match rest.rfind('"') {
            Some(at) => (&rest[..at], Some(rest[at + 1..].to_string())),
            None => (text, None),
        },
        None => (text, None),
    };

    let mut words = head.split_whitespace();
    let data_type = words.next()?;
    let name = words.next()?;

    let mut column = ErColumn::new(String::new(), name).with_type(data_type);

    for word in words.flat_map(|w| w.split(',')) {
        let word = word.trim().to_lowercase();
        if word.is_empty() {
            continue;
        }
        if !KEYS.contains(&word.as_str()) {
            return None;
        }

        match word.as_str() {
            "pk" => column = column.as_primary_key(),
            "fk" => column.foreign_key = true,
            _ => column.unique = true,
        }
    }

    column.note = note.filter(|n| !n.is_empty());

    Some(column)
}

/// Quita la palabra si la linea empieza por ella y trae algo detras.
///
/// `title` a secas no es un titulo, es una linea suelta, y lo mismo para
/// `direction`: aqui las dos palabras piden argumento.
fn keyword<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    strip_word(text, word).filter(|rest| !rest.is_empty())
}

fn direction_of(text: &str) -> Option<LayoutDirection> {
    match text.to_uppercase().as_str() {
        "LR" => Some(LayoutDirection::Right),
        "RL" => Some(LayoutDirection::Left),
        "TB" | "TD" => Some(LayoutDirection::Down),
        "BT" => Some(LayoutDirection::Up),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
