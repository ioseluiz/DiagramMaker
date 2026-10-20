//! Declaraciones de elementos de un diagrama de componentes.
//!
//! PlantUML nombra la forma con una palabra clave —`component`, `database`,
//! `queue`— y admite muchas variantes de la misma linea:
//!
//! ```text
//! [Portal web]
//! [Portal web] as web
//! component "API REST" as api
//! database db #LightBlue
//! package "Capa publica" {
//! rectangle "Caja" as c <<servicio>>
//! ```
//!
//! Todas se leen igual: palabra clave, nombre visible y, detras, modificadores
//! en cualquier orden.
//!
//! Hay tres formas cortas que se escriben sin palabra clave, una por cada
//! dibujo que sale lo bastante a menudo como para merecerla: `[Componente]`,
//! `(Caso de uso)` y `:Actor:`.

use crate::model::NodeShape;

/// Un elemento declarado en una linea.
#[derive(Debug, Clone, PartialEq)]
pub struct Declared {
    pub shape: NodeShape,
    /// Texto que se dibuja dentro de la caja.
    pub label: String,
    /// Con que nombre se le referencia en las relaciones: el alias si lo hay,
    /// y si no el propio nombre visible.
    pub name: String,
    pub fill: Option<String>,
    /// La linea acababa en `{`: lo que venga detras va dentro.
    pub opens_block: bool,
    /// Lo que quedo sin entender al final de la linea.
    pub trailing: String,
}

/// Palabras clave y la forma que dibuja cada una.
///
/// PlantUML tiene mas formas que el modelo, asi que varias caen en la misma:
/// `storage` y `database` son los dos un cilindro, y `node`, `artifact` o
/// `card` acaban siendo cajas. Es una perdida consciente —el diagrama sigue
/// diciendo lo mismo— y no al reves: nada se inventa.
const KEYWORDS: &[(&str, NodeShape)] = &[
    ("component", NodeShape::Rectangle),
    ("rectangle", NodeShape::Rectangle),
    ("node", NodeShape::Rectangle),
    ("artifact", NodeShape::Rectangle),
    ("agent", NodeShape::Rectangle),
    ("file", NodeShape::Rectangle),
    ("collections", NodeShape::Rectangle),
    ("boundary", NodeShape::Rectangle),
    ("control", NodeShape::Rectangle),
    ("entity", NodeShape::Rectangle),
    ("card", NodeShape::RoundedRect),
    ("cloud", NodeShape::RoundedRect),
    ("database", NodeShape::Cylinder),
    ("storage", NodeShape::Cylinder),
    ("queue", NodeShape::Hexagon),
    ("interface", NodeShape::Ellipse),
    ("usecase", NodeShape::Ellipse),
    ("circle", NodeShape::Ellipse),
    ("actor", NodeShape::Actor),
    ("package", NodeShape::Package),
    ("folder", NodeShape::Package),
    ("frame", NodeShape::Package),
];

pub fn shape_for(keyword: &str) -> Option<NodeShape> {
    KEYWORDS
        .iter()
        .find(|(word, _)| *word == keyword)
        .map(|(_, shape)| *shape)
}

/// Palabra clave con la que escribir cada forma. La usa el emisor.
///
/// Las formas que PlantUML no tiene —rombo, paralelogramo— salen como
/// `rectangle`: se pierde el contorno, pero no el elemento ni sus relaciones.
pub fn keyword_for(shape: NodeShape) -> &'static str {
    match shape {
        NodeShape::Cylinder => "database",
        NodeShape::Hexagon => "queue",
        NodeShape::Ellipse => "usecase",
        NodeShape::Actor => "actor",
        NodeShape::RoundedRect => "card",
        NodeShape::Package => "package",
        _ => "rectangle",
    }
}

/// Lee una linea de declaracion. `None` si no lo es.
pub fn parse(text: &str) -> Option<Declared> {
    let mut head = text.trim();

    let opens_block = head.ends_with('{');
    if opens_block {
        head = head[..head.len() - 1].trim_end();
    }

    // Las formas cortas no llevan palabra clave: el propio signo la dice.
    let (shape, rest) = if let Some(shape) = shorthand_shape(head) {
        (shape, head)
    } else {
        let word = head.split_whitespace().next()?;
        let shape = shape_for(&word.to_lowercase())?;
        (shape, head[word.len()..].trim_start())
    };

    let (label, mut tail) = read_token(rest)?;
    if label.is_empty() {
        return None;
    }

    let mut name = label.clone();
    let mut fill = None;

    loop {
        let current = tail.trim_start();
        if current.is_empty() {
            break;
        }

        if let Some(after) = strip_word(current, "as") {
            let Some((alias, next)) = read_token(after) else {
                break;
            };
            name = alias;
            tail = next;
            continue;
        }

        // `<<servicio>>`: el estereotipo es decorativo y no cambia el
        // diagrama, asi que se consume y se olvida.
        if let Some(after) = current.strip_prefix("<<") {
            let Some(end) = after.find(">>") else {
                break;
            };
            tail = &after[end + 2..];
            continue;
        }

        if let Some(after) = current.strip_prefix('#') {
            let end = after.find(char::is_whitespace).unwrap_or(after.len());
            let color = &after[..end];
            // `#line:red;back:blue` es un estilo compuesto; solo se entiende
            // el color plano.
            if !color.is_empty() && !color.contains(':') {
                fill = Some(color.to_string());
            }
            tail = &after[end..];
            continue;
        }

        break;
    }

    Some(Declared {
        // Un bloque es un contenedor, se haya escrito con la palabra que se
        // haya escrito: dibujarlo como paquete es lo unico que deja ver que
        // hay cosas dentro.
        shape: if opens_block {
            NodeShape::Package
        } else {
            shape
        },
        label,
        name,
        fill,
        opens_block,
        trailing: tail.trim().to_string(),
    })
}

/// Forma que declara una de las tres formas cortas, si el texto empieza por
/// una de ellas.
///
/// Solo mira el signo que abre, porque detras puede venir cualquier cosa:
/// `[Portal web] as web #red` es una declaracion y `(A) --> (B)` una relacion,
/// y las dos empiezan por su signo. De que el signo cierre ya se encarga
/// `read_token`, que es quien lee el nombre.
pub fn shorthand_shape(text: &str) -> Option<NodeShape> {
    match text.trim_start().as_bytes().first()? {
        b'[' => Some(NodeShape::Rectangle),
        b'(' => Some(NodeShape::Ellipse),
        b':' => Some(NodeShape::Actor),
        _ => None,
    }
}

/// Lee el siguiente trozo: una cadena entre comillas, un nombre entre
/// corchetes o una palabra suelta. Devuelve su contenido y lo que queda.
pub fn read_token(text: &str) -> Option<(String, &str)> {
    let text = text.trim_start();
    let close = match text.as_bytes().first()? {
        b'"' => '"',
        b'[' => ']',
        b'(' => ')',
        b':' => ':',
        _ => {
            let end = text.find(char::is_whitespace).unwrap_or(text.len());
            return Some((text[..end].to_string(), &text[end..]));
        }
    };

    let end = text[1..].find(close)? + 1;
    Some((text[1..end].trim().to_string(), &text[end + 1..]))
}

/// Consume una palabra suelta al principio, sin distinguir mayusculas.
pub fn strip_word<'a>(text: &'a str, word: &str) -> Option<&'a str> {
    if text.len() < word.len() || !text[..word.len()].eq_ignore_ascii_case(word) {
        return None;
    }

    let rest = &text[word.len()..];
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }

    Some(rest)
}

/// Quita los signos que rodean a la referencia de un elemento dentro de una
/// relacion: los de las tres formas cortas y las comillas.
pub fn unwrap_reference(text: &str) -> &str {
    let text = text.trim();

    for (open, close) in [('[', ']'), ('(', ')'), (':', ':'), ('"', '"')] {
        if let Some(inner) = text.strip_prefix(open).and_then(|t| t.strip_suffix(close)) {
            return inner.trim();
        }
    }

    text
}
