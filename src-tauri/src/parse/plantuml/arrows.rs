//! Reconocimiento de las flechas de PlantUML.
//!
//! Una relacion se escribe `A flecha B : etiqueta`, y la flecha es un dibujo:
//! `-->`, `..>`, `o--`, `--|>`. Clasificarla es facil; lo dificil es
//! *encontrarla*, porque el cuerpo admite cosas dentro —una pista de
//! direccion (`-up->`) o un estilo (`-[#red]->`)— y porque los nombres de los
//! elementos llevan guiones (`api-rest`) y corchetes (`[Portal web]`).
//!
//! De ahi las dos reglas que sostienen este modulo:
//!
//! 1. Un `[` solo abre el nombre de un componente si no viene pegado a un
//!    guion o un punto; pegado, es el estilo de la flecha.
//! 2. Una flecha sin ninguna punta (`-`, `--`, `..`) tiene que estar
//!    despegada de los nombres, o llevar el cuerpo doble. Sin esa regla,
//!    `api-rest` seria una relacion entre `api` y `rest`; con ella,
//!    `banco--parte` sigue siendolo, porque ningun identificador lleva dos
//!    guiones seguidos.

use crate::model::EdgeKind;

/// Lo que se dibuja en un extremo de la flecha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Head {
    /// `>` o `<`
    Open,
    /// `|>` o `<|`
    Triangle,
    /// `o`
    Hollow,
    /// `*`
    Solid,
}

/// Una flecha reconocida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrow {
    pub kind: EdgeKind,
    pub dashed: bool,
    /// La punta miraba al elemento de la izquierda. Quien llame tiene que
    /// intercambiar los extremos: en el modelo la punta va siempre al destino.
    pub reversed: bool,
}

/// Pistas de direccion que PlantUML admite dentro del cuerpo de la flecha.
/// No se conservan: la direccion la decide el auto-layout.
const DIRECTIONS: &[&str] = &["up", "down", "left", "right", "u", "d", "l", "r"];

/// Busca la flecha de una linea. Devuelve el rango que ocupa y que es.
pub fn find(text: &str) -> Option<(usize, usize, Arrow)> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut quoted = false;
    let mut in_name = false;

    while index < bytes.len() {
        let c = bytes[index];

        if c == b'"' {
            quoted = !quoted;
            index += 1;
            continue;
        }
        if quoted {
            index += 1;
            continue;
        }

        // Regla 1: `[` pegado a un guion es estilo de flecha, no un nombre.
        if c == b'[' && !in_name && !glued_to_body(bytes, index) {
            in_name = true;
            index += 1;
            continue;
        }
        if c == b']' && in_name {
            in_name = false;
            index += 1;
            continue;
        }
        if in_name {
            index += 1;
            continue;
        }

        if matches!(c, b'-' | b'.' | b'<' | b'o' | b'*')
            && let Some((end, arrow)) = read(text, index)
        {
            return Some((index, end, arrow));
        }

        index += 1;
    }

    None
}

/// `true` si el caracter anterior es parte del cuerpo de una flecha.
fn glued_to_body(bytes: &[u8], index: usize) -> bool {
    index > 0 && matches!(bytes[index - 1], b'-' | b'.')
}

/// Intenta leer una flecha que empieza en `start`.
fn read(text: &str, start: usize) -> Option<(usize, Arrow)> {
    let bytes = text.as_bytes();
    let mut index = start;

    let left = read_left_head(bytes, &mut index)?;

    let body_start = index;
    // Los puntos del cuerpo cambian el *tipo* de relacion: `..>` es una
    // dependencia. Un `[dashed]` solo cambia como se dibuja la linea.
    let mut dotted = false;
    let mut styled = false;

    while index < bytes.len() {
        match bytes[index] {
            b'-' => index += 1,
            b'.' => {
                dotted = true;
                index += 1;
            }
            // `-[#red]->`, `-[dashed]->`: estilo dentro del cuerpo.
            b'[' => {
                let close = text[index..].find(']')? + index;
                let inside = text[index + 1..close].to_lowercase();
                if inside.contains("dashed") || inside.contains("dotted") {
                    styled = true;
                }
                index = close + 1;
            }
            c if c.is_ascii_alphabetic() => {
                let word_end = index
                    + text[index..]
                        .find(|c: char| !c.is_ascii_alphabetic())
                        .unwrap_or(text.len() - index);
                let word = text[index..word_end].to_lowercase();

                // Una palabra solo pertenece al cuerpo si es una direccion y
                // detras sigue habiendo flecha. Si no, es el nombre del
                // elemento de la derecha y la flecha acaba aqui.
                if !DIRECTIONS.contains(&word.as_str())
                    || !matches!(bytes.get(word_end), Some(b'-') | Some(b'.'))
                {
                    break;
                }
                index = word_end;
            }
            _ => break,
        }
    }

    if index == body_start {
        return None;
    }

    let right = read_right_head(bytes, &mut index);

    // Regla 2: sin puntas, o la flecha esta despegada de los nombres, o su
    // cuerpo es doble. Un solo guion pegado (`api-rest`) es parte del nombre;
    // dos (`banco--parte`) no los lleva ningun identificador.
    if left.is_none()
        && right.is_none()
        && index - body_start < 2
        && !(boundary_before(bytes, start) && boundary_after(bytes, index))
    {
        return None;
    }

    let (kind, reversed) = classify(left, right, dotted);

    Some((
        index,
        Arrow {
            kind,
            dashed: dotted || styled,
            reversed,
        },
    ))
}

/// Lee `<`, `<|`, `o` o `*` al principio de la flecha.
///
/// Devuelve `None` —fallando la lectura entera— si hay una punta donde no
/// cabe: una `o` pegada al final de un nombre (`banco--parte`) no es una
/// agregacion.
fn read_left_head(bytes: &[u8], index: &mut usize) -> Option<Option<Head>> {
    let c = bytes[*index];
    if !matches!(c, b'<' | b'o' | b'*') {
        return Some(None);
    }
    if !boundary_before(bytes, *index) {
        return None;
    }

    let head = match c {
        b'<' if bytes.get(*index + 1) == Some(&b'|') => {
            *index += 1;
            Head::Triangle
        }
        b'<' => Head::Open,
        b'o' => Head::Hollow,
        _ => Head::Solid,
    };
    *index += 1;

    Some(Some(head))
}

/// Lee `>`, `|>`, `o` o `*` al final de la flecha.
fn read_right_head(bytes: &[u8], index: &mut usize) -> Option<Head> {
    match bytes.get(*index)? {
        b'|' if bytes.get(*index + 1) == Some(&b'>') => {
            *index += 2;
            Some(Head::Triangle)
        }
        b'>' => {
            *index += 1;
            Some(Head::Open)
        }
        // Una `o` solo es punta si detras no sigue una palabra: en `--otro`
        // la `o` es del nombre.
        c @ (b'o' | b'*') if boundary_after(bytes, *index + 1) => {
            let head = if *c == b'o' {
                Head::Hollow
            } else {
                Head::Solid
            };
            *index += 1;
            Some(head)
        }
        _ => None,
    }
}

/// `true` si delante de `index` acaba algo: el principio de la linea, un
/// espacio, o el cierre del nombre de un elemento.
fn boundary_before(bytes: &[u8], index: usize) -> bool {
    match index.checked_sub(1).map(|i| bytes[i]) {
        None => true,
        Some(c) => c.is_ascii_whitespace() || matches!(c, b']' | b'"' | b')'),
    }
}

/// `true` si en `index` empieza algo distinto de la flecha.
fn boundary_after(bytes: &[u8], index: usize) -> bool {
    match bytes.get(index) {
        None => true,
        Some(c) => c.is_ascii_whitespace() || matches!(c, b'[' | b'"' | b'('),
    }
}

/// Que relacion dibuja cada combinacion de puntas.
///
/// El rombo de agregacion y composicion va en el origen, tanto en PlantUML
/// como en el modelo; la flecha abierta va en el destino. Por eso una punta
/// a la izquierda invierte la relacion en un caso y no en el otro.
fn classify(left: Option<Head>, right: Option<Head>, dashed: bool) -> (EdgeKind, bool) {
    let pointed = if dashed {
        EdgeKind::Dependency
    } else {
        EdgeKind::Directed
    };
    let triangle = if dashed {
        EdgeKind::Realization
    } else {
        EdgeKind::Inheritance
    };

    match (left, right) {
        (_, Some(Head::Triangle)) => (triangle, false),
        (Some(Head::Triangle), _) => (triangle, true),
        (Some(Head::Hollow), _) => (EdgeKind::Aggregation, false),
        (_, Some(Head::Hollow)) => (EdgeKind::Aggregation, true),
        (Some(Head::Solid), _) => (EdgeKind::Composition, false),
        (_, Some(Head::Solid)) => (EdgeKind::Composition, true),
        (_, Some(Head::Open)) => (pointed, false),
        (Some(Head::Open), _) => (pointed, true),
        (None, None) => (EdgeKind::Association, false),
    }
}

/// Como escribir cada relacion. Lo usa el emisor.
///
/// `Flow` y `Directed` se dibujan igual: PlantUML no distingue un flujo de
/// datos de una flecha corriente en un diagrama de componentes.
pub fn arrow_for(kind: EdgeKind, dashed: bool) -> &'static str {
    match kind {
        EdgeKind::Inheritance => "--|>",
        EdgeKind::Realization => "..|>",
        EdgeKind::Aggregation => "o--",
        EdgeKind::Composition => "*--",
        EdgeKind::Dependency => "..>",
        EdgeKind::Directed | EdgeKind::Flow if dashed => "-[dashed]->",
        EdgeKind::Directed | EdgeKind::Flow => "-->",
        EdgeKind::Association if dashed => "..",
        EdgeKind::Association => "--",
    }
}
