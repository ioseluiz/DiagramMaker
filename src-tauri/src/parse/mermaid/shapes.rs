//! Reconocimiento de formas y flechas de Mermaid.
//!
//! En Mermaid la forma de un nodo no se nombra: se deduce de los signos que
//! rodean su etiqueta. `A[x]` es un rectangulo, `A([x])` un estadio y
//! `A[(x)]` un cilindro. Los delimitadores se solapan entre si, asi que el
//! orden en que se prueban importa: los mas largos primero, o `([` se leeria
//! como `(`.

use crate::model::{EdgeKind, NodeShape};

/// Una forma reconocida: su contorno y la etiqueta que llevaba dentro.
#[derive(Debug, Clone, PartialEq)]
pub struct Shaped {
    pub shape: NodeShape,
    pub label: String,
}

/// Un nodo tal y como aparece en el codigo.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    /// Identificador con el que se le referencia.
    pub name: String,
    /// Forma y etiqueta, si la linea las declaraba.
    pub shape: Option<Shaped>,
}

/// Una flecha entre dos nodos.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    pub kind: EdgeKind,
    pub label: Option<String>,
    pub dashed: bool,
    pub thick: bool,
}

/// Una linea completa: los nodos y las flechas que los unen.
#[derive(Debug, Clone, PartialEq)]
pub struct Chain {
    pub nodes: Vec<Declaration>,
    /// Siempre uno menos que `nodes`.
    pub links: Vec<Link>,
}

/// Delimitadores de forma, de mas largo a mas corto.
///
/// El orden es la parte delicada: `([` empieza por `(`, asi que si se probara
/// el corto primero un estadio se leeria como un redondeado y sobraria un
/// corchete.
const DELIMITERS: &[(&str, &str, NodeShape)] = &[
    ("[(", ")]", NodeShape::Cylinder),
    ("([", "])", NodeShape::RoundedRect),
    ("[[", "]]", NodeShape::Rectangle),
    ("((", "))", NodeShape::Ellipse),
    ("{{", "}}", NodeShape::Hexagon),
    ("[/", "/]", NodeShape::Parallelogram),
    ("[\\", "\\]", NodeShape::Parallelogram),
    (">", "]", NodeShape::Parallelogram),
    ("[", "]", NodeShape::Rectangle),
    ("(", ")", NodeShape::RoundedRect),
    ("{", "}", NodeShape::Diamond),
];

/// Separa `id[Etiqueta]` en su identificador y su forma.
///
/// Devuelve `None` si no hay delimitadores: el texto es solo un
/// identificador, y su forma la decide quien llame.
pub fn split_declaration(text: &str) -> Option<(&str, Shaped)> {
    let text = text.trim();

    for (open, close, shape) in DELIMITERS {
        let Some(start) = text.find(open) else {
            continue;
        };
        if start == 0 || !text.ends_with(close) {
            continue;
        }

        // Lo que queda delante tiene que ser un identificador. Sin esta
        // comprobacion, el delimitador `>` de la forma asimetrica (`a>x]`)
        // se dispara dentro de un `<br/>` o de una flecha `-->`, y parte la
        // linea por donde no es.
        let name = text[..start].trim();
        if !is_identifier(name) {
            continue;
        }

        let label_start = start + open.len();
        let label_end = text.len() - close.len();
        if label_end < label_start {
            continue;
        }

        return Some((
            name,
            Shaped {
                shape: *shape,
                label: clean_label(&text[label_start..label_end]),
            },
        ));
    }

    None
}

/// `true` si el texto puede ser el identificador de un nodo.
///
/// Mermaid los escribe con letras, digitos, guiones y guiones bajos. Nada de
/// espacios ni de signos: esos ya forman parte de otra cosa.
fn is_identifier(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

/// Quita las comillas y los `<br>` de una etiqueta.
fn clean_label(raw: &str) -> String {
    let trimmed = raw.trim();
    let unquoted = trimmed
        .strip_prefix('"')
        .and_then(|t| t.strip_suffix('"'))
        .unwrap_or(trimmed);

    unquoted
        .replace("<br/>", "\n")
        .replace("<br>", "\n")
        .trim()
        .to_string()
}

/// Flechas reconocidas, de mas larga a mas corta por el mismo motivo que los
/// delimitadores: `-.->` empieza como `-.-`.
const ARROWS: &[(&str, EdgeKind, bool, bool)] = &[
    ("<-->", EdgeKind::Association, false, false),
    ("-.->", EdgeKind::Dependency, true, false),
    ("<-.->", EdgeKind::Dependency, true, false),
    ("==>", EdgeKind::Directed, false, true),
    ("-.-", EdgeKind::Association, true, false),
    ("===", EdgeKind::Association, false, true),
    ("-->", EdgeKind::Directed, false, false),
    ("---", EdgeKind::Association, false, false),
    ("--o", EdgeKind::Aggregation, false, false),
    ("--x", EdgeKind::Directed, false, false),
];

/// Busca la flecha mas larga que empieza en `at`.
fn arrow_at(text: &str, at: usize) -> Option<(&'static str, EdgeKind, bool, bool)> {
    let rest = &text[at..];

    ARROWS
        .iter()
        .filter(|(pattern, ..)| rest.starts_with(pattern))
        .max_by_key(|(pattern, ..)| pattern.len())
        .copied()
}

/// Parte una linea en nodos y flechas.
///
/// Acepta cadenas: `A --> B --> C` declara dos relaciones. Las etiquetas de
/// flecha se escriben de dos maneras —`A -->|texto| B` y `A -- texto --> B`—
/// y las dos se entienden.
pub fn split_chain(text: &str) -> Result<Chain, String> {
    let mut nodes: Vec<Declaration> = Vec::new();
    let mut links: Vec<Link> = Vec::new();

    let mut current = String::new();
    let mut depth = 0usize;
    let mut quoted = false;
    let mut index = 0usize;

    let bytes = text.as_bytes();

    while index < text.len() {
        let c = bytes[index] as char;

        if c == '"' {
            quoted = !quoted;
            current.push(c);
            index += 1;
            continue;
        }

        if !quoted {
            match c {
                '[' | '(' | '{' => depth += 1,
                ']' | ')' | '}' => depth = depth.saturating_sub(1),
                _ => {}
            }

            // Una flecha solo cuenta fuera de los delimitadores de etiqueta:
            // `A[a-->b]` es un nodo, no dos.
            if depth == 0
                && let Some((pattern, kind, dashed, thick)) = arrow_at(text, index)
            {
                nodes.push(declaration(&current)?);
                current.clear();
                index += pattern.len();

                let label = read_arrow_label(text, &mut index);
                links.push(Link {
                    kind,
                    label,
                    dashed,
                    thick,
                });
                continue;
            }

            // Forma `A -- texto --> B`: el texto va entre dos guiones.
            if depth == 0
                && text[index..].starts_with("--")
                && let Some(link) = read_dashed_label(text, &mut index)
            {
                nodes.push(declaration(&current)?);
                current.clear();
                links.push(link);
                continue;
            }
        }

        current.push(c);
        index += 1;
    }

    nodes.push(declaration(&current)?);

    if nodes.len() != links.len() + 1 {
        return Err("La linea tiene flechas sueltas sin nodo a un lado.".into());
    }

    Ok(Chain { nodes, links })
}

/// Lee `|texto|` detras de una flecha, si lo hay.
fn read_arrow_label(text: &str, index: &mut usize) -> Option<String> {
    let rest = text[*index..].trim_start();
    let skipped = text[*index..].len() - rest.len();

    let body = rest.strip_prefix('|')?;
    let end = body.find('|')?;

    *index += skipped + 1 + end + 1;
    Some(clean_label(&body[..end]))
}

/// Lee `-- texto -->`, la otra forma de etiquetar una flecha.
///
/// Devuelve `None` si lo que viene no encaja, para que el llamante lo trate
/// como texto normal en lugar de perderlo.
fn read_dashed_label(text: &str, index: &mut usize) -> Option<Link> {
    let after_dashes = *index + 2;
    let rest = &text[after_dashes..];

    // El texto va hasta los siguientes guiones, que ya forman la flecha.
    let end = rest.find("--")?;
    let label = rest[..end].trim();

    // Sin texto entre medias, esto era una flecha normal: que la lea el otro
    // camino.
    if label.is_empty() || label.contains(['[', ']', '|']) {
        return None;
    }

    let arrow_start = after_dashes + end;
    let (pattern, kind, dashed, thick) = arrow_at(text, arrow_start)?;

    *index = arrow_start + pattern.len();

    Some(Link {
        kind,
        label: Some(clean_label(label)),
        dashed,
        thick,
    })
}

fn declaration(raw: &str) -> Result<Declaration, String> {
    let text = raw.trim();
    if text.is_empty() {
        return Err("Falta un nodo a un lado de la flecha.".into());
    }

    match split_declaration(text) {
        Some((name, shape)) if !name.is_empty() => Ok(Declaration {
            name: name.to_string(),
            shape: Some(shape),
        }),
        // `[Solo etiqueta]` sin identificador delante no es valido.
        Some(_) => Err(format!(
            "`{text}` no tiene identificador antes de la forma."
        )),
        None => Ok(Declaration {
            name: text.to_string(),
            shape: None,
        }),
    }
}

/// Delimitadores con los que escribir cada forma. Lo usa el emisor.
pub fn delimiters_for(shape: NodeShape) -> (&'static str, &'static str) {
    match shape {
        NodeShape::Cylinder => ("[(", ")]"),
        NodeShape::Ellipse => ("((", "))"),
        NodeShape::Hexagon => ("{{", "}}"),
        NodeShape::Parallelogram => ("[/", "/]"),
        NodeShape::Diamond => ("{", "}"),
        NodeShape::RoundedRect => ("(", ")"),
        _ => ("[", "]"),
    }
}
