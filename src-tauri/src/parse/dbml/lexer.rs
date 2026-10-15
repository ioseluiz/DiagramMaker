//! Tokenizador de DBML.
//!
//! DBML es un lenguaje de bloques con llaves, corchetes de ajustes y tres
//! clases de literal de texto: comillas simples, comillas triples (`'''`) para
//! texto multilinea y acentos graves para expresiones SQL (`` `now()` ``).

use crate::model::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// Identificador o palabra clave: `Table`, `users`, `varchar`.
    Word(String),
    /// Texto entre comillas, ya sin ellas.
    Text(String),
    /// Expresion entre acentos graves, ya sin ellos.
    Expression(String),
    Number(String),
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Comma,
    Colon,
    Dot,
    /// Operadores de relacion: `>`, `<`, `-`, `<>`.
    Op(String),
    /// Salto de linea. DBML separa columnas por lineas, asi que importan.
    Newline,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    /// Texto de un `Word` o `Text`, que es como se nombran tablas y columnas.
    pub fn name(&self) -> Option<&str> {
        match &self.kind {
            TokenKind::Word(w) | TokenKind::Text(w) => Some(w),
            _ => None,
        }
    }

    pub fn is(&self, kind: &TokenKind) -> bool {
        &self.kind == kind
    }

    /// Compara con una palabra clave sin distinguir mayusculas: DBML acepta
    /// `Table`, `table` y `TABLE`.
    pub fn is_keyword(&self, keyword: &str) -> bool {
        matches!(&self.kind, TokenKind::Word(w) if w.eq_ignore_ascii_case(keyword))
    }
}

/// Error lexico, con su posicion.
#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

pub struct Lexer<'a> {
    source: &'a [u8],
    pos: usize,
    line: u32,
    column: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Lexer {
            source: source.as_bytes(),
            pos: 0,
            line: 1,
            column: 1,
        }
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.column)
    }

    fn peek(&self) -> Option<u8> {
        self.source.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.source.get(self.pos + offset).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        if c == b'\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// Consume espacios y comentarios, pero **no** saltos de linea: el parser
    /// los necesita para saber donde acaba una columna.
    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            match self.peek() {
                Some(b' ' | b'\t' | b'\r') => {
                    self.bump();
                }
                Some(b'/') if self.peek_at(1) == Some(b'/') => {
                    while !matches!(self.peek(), None | Some(b'\n')) {
                        self.bump();
                    }
                }
                Some(b'/') if self.peek_at(1) == Some(b'*') => {
                    let start = self.span();
                    self.bump();
                    self.bump();
                    loop {
                        match self.peek() {
                            None => {
                                return Err(LexError {
                                    message: "Comentario /* sin cerrar.".into(),
                                    span: start,
                                });
                            }
                            Some(b'*') if self.peek_at(1) == Some(b'/') => {
                                self.bump();
                                self.bump();
                                break;
                            }
                            _ => {
                                self.bump();
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn read_quoted(&mut self, quote: u8) -> Result<String, LexError> {
        let start = self.span();

        // Comillas triples: texto multilinea.
        let triple =
            quote == b'\'' && self.peek_at(1) == Some(b'\'') && self.peek_at(2) == Some(b'\'');

        if triple {
            for _ in 0..3 {
                self.bump();
            }
        } else {
            self.bump();
        }

        let mut out = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        message: "Texto entre comillas sin cerrar.".into(),
                        span: start,
                    });
                }
                Some(b'\\') => {
                    self.bump();
                    match self.bump() {
                        Some(b'n') => out.push('\n'),
                        Some(b't') => out.push('\t'),
                        Some(c) => out.push(c as char),
                        None => {}
                    }
                }
                Some(c) if !triple && c == quote => {
                    self.bump();
                    break;
                }
                Some(b'\'')
                    if triple
                        && self.peek_at(1) == Some(b'\'')
                        && self.peek_at(2) == Some(b'\'') =>
                {
                    for _ in 0..3 {
                        self.bump();
                    }
                    break;
                }
                _ => {
                    let start_byte = self.pos;
                    self.bump();
                    // Avanzar hasta completar el caracter UTF-8.
                    while self.peek().is_some_and(|c| (0x80..0xC0).contains(&c)) {
                        self.bump();
                    }
                    out.push_str(&String::from_utf8_lossy(&self.source[start_byte..self.pos]));
                }
            }
        }

        if triple {
            // El texto multilinea de DBML suele venir indentado; se quita el
            // sangrado comun para que la nota no arrastre los espacios del
            // codigo fuente.
            return Ok(dedent(&out));
        }

        Ok(out)
    }

    fn read_word(&mut self) -> String {
        let start = self.pos;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c >= 0x80)
        {
            self.bump();
        }
        String::from_utf8_lossy(&self.source[start..self.pos]).into_owned()
    }

    fn read_number(&mut self) -> String {
        let start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_digit() || c == b'.') {
            self.bump();
        }
        String::from_utf8_lossy(&self.source[start..self.pos]).into_owned()
    }

    /// Convierte toda la entrada en tokens.
    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut out = Vec::new();

        loop {
            self.skip_trivia()?;
            let span = self.span();

            let Some(c) = self.peek() else {
                out.push(Token {
                    kind: TokenKind::End,
                    span,
                });
                return Ok(out);
            };

            let kind = match c {
                b'\n' => {
                    self.bump();
                    TokenKind::Newline
                }
                b'{' => {
                    self.bump();
                    TokenKind::LBrace
                }
                b'}' => {
                    self.bump();
                    TokenKind::RBrace
                }
                b'[' => {
                    self.bump();
                    TokenKind::LBracket
                }
                b']' => {
                    self.bump();
                    TokenKind::RBracket
                }
                b'(' => {
                    self.bump();
                    TokenKind::LParen
                }
                b')' => {
                    self.bump();
                    TokenKind::RParen
                }
                b',' => {
                    self.bump();
                    TokenKind::Comma
                }
                b':' => {
                    self.bump();
                    TokenKind::Colon
                }
                b'.' => {
                    self.bump();
                    TokenKind::Dot
                }
                b'\'' | b'"' => TokenKind::Text(self.read_quoted(c)?),
                b'`' => {
                    self.bump();
                    let start = self.pos;
                    while !matches!(self.peek(), None | Some(b'`')) {
                        self.bump();
                    }
                    let text = String::from_utf8_lossy(&self.source[start..self.pos]).into_owned();
                    if self.peek().is_none() {
                        return Err(LexError {
                            message: "Expresion entre ` sin cerrar.".into(),
                            span,
                        });
                    }
                    self.bump();
                    TokenKind::Expression(text)
                }
                b'<' if self.peek_at(1) == Some(b'>') => {
                    self.bump();
                    self.bump();
                    TokenKind::Op("<>".into())
                }
                b'<' | b'>' | b'-' => {
                    self.bump();
                    TokenKind::Op((c as char).to_string())
                }
                // Color hexadecimal: `#3498db`. Se lee entero como una palabra
                // para que llegue de una pieza al ajuste que lo usa.
                b'#' => {
                    let start = self.pos;
                    self.bump();
                    while self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
                        self.bump();
                    }
                    TokenKind::Word(
                        String::from_utf8_lossy(&self.source[start..self.pos]).into_owned(),
                    )
                }
                c if c.is_ascii_digit() => TokenKind::Number(self.read_number()),
                c if c.is_ascii_alphabetic() || c == b'_' || c >= 0x80 => {
                    TokenKind::Word(self.read_word())
                }
                other => {
                    self.bump();
                    return Err(LexError {
                        message: format!("Caracter inesperado `{}`.", other as char),
                        span,
                    });
                }
            };

            out.push(Token { kind, span });
        }
    }
}

/// Quita el sangrado comun de un texto multilinea.
fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();

    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);

    lines
        .iter()
        .map(|l| {
            if l.len() >= indent {
                &l[indent..]
            } else {
                l.trim()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(source: &str) -> Vec<TokenKind> {
        Lexer::new(source)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Newline | TokenKind::End))
            .collect()
    }

    #[test]
    fn reconoce_palabras_y_signos() {
        assert_eq!(
            kinds("Table users {"),
            [
                TokenKind::Word("Table".into()),
                TokenKind::Word("users".into()),
                TokenKind::LBrace,
            ]
        );
    }

    #[test]
    fn los_saltos_de_linea_son_tokens() {
        let tokens = Lexer::new("a\nb").tokenize().unwrap();

        assert!(tokens.iter().any(|t| t.kind == TokenKind::Newline));
    }

    #[test]
    fn ignora_comentarios_de_linea() {
        assert_eq!(
            kinds("users // esto no cuenta"),
            [TokenKind::Word("users".into())]
        );
    }

    #[test]
    fn ignora_comentarios_de_bloque() {
        assert_eq!(
            kinds("a /* varias\nlineas */ b"),
            [TokenKind::Word("a".into()), TokenKind::Word("b".into())]
        );
    }

    #[test]
    fn un_comentario_de_bloque_sin_cerrar_es_un_error_situado() {
        let err = Lexer::new("a\n/* nunca cierra").tokenize().unwrap_err();

        assert_eq!(err.span.line, 2);
        assert!(err.message.contains("sin cerrar"));
    }

    #[test]
    fn lee_texto_entre_comillas_simples_y_dobles() {
        assert_eq!(
            kinds("'hola' \"adios\""),
            [
                TokenKind::Text("hola".into()),
                TokenKind::Text("adios".into())
            ]
        );
    }

    #[test]
    fn lee_texto_multilinea_entre_comillas_triples_y_le_quita_el_sangrado() {
        let source = "'''\n    primera\n    segunda\n    '''";

        assert_eq!(kinds(source), [TokenKind::Text("primera\nsegunda".into())]);
    }

    #[test]
    fn lee_expresiones_entre_acentos_graves() {
        assert_eq!(kinds("`now()`"), [TokenKind::Expression("now()".into())]);
    }

    #[test]
    fn un_texto_sin_cerrar_es_un_error_situado() {
        let err = Lexer::new("nota: 'sin cerrar").tokenize().unwrap_err();

        assert!(err.message.contains("sin cerrar"));
        assert_eq!(err.span.line, 1);
    }

    #[test]
    fn distingue_los_operadores_de_relacion() {
        assert_eq!(
            kinds("> < - <>"),
            [
                TokenKind::Op(">".into()),
                TokenKind::Op("<".into()),
                TokenKind::Op("-".into()),
                TokenKind::Op("<>".into()),
            ]
        );
    }

    #[test]
    fn el_tipo_con_tamano_se_parte_en_sus_piezas() {
        assert_eq!(
            kinds("varchar(255)"),
            [
                TokenKind::Word("varchar".into()),
                TokenKind::LParen,
                TokenKind::Number("255".into()),
                TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn lee_un_color_hexadecimal_de_una_pieza() {
        assert_eq!(kinds("#3498db"), [TokenKind::Word("#3498db".into())]);
    }

    #[test]
    fn acepta_identificadores_con_acentos() {
        assert_eq!(kinds("año"), [TokenKind::Word("año".into())]);
    }

    #[test]
    fn el_texto_con_acentos_no_se_corrompe() {
        assert_eq!(
            kinds("'descripción ñ'"),
            [TokenKind::Text("descripción ñ".into())]
        );
    }

    #[test]
    fn las_posiciones_apuntan_a_la_linea_correcta() {
        let tokens = Lexer::new("Table\n  users").tokenize().unwrap();
        let users = tokens.iter().find(|t| t.name() == Some("users")).unwrap();

        assert_eq!(users.span, Span::new(2, 3));
    }

    #[test]
    fn un_caracter_desconocido_se_reporta_con_su_posicion() {
        let err = Lexer::new("Table users @").tokenize().unwrap_err();

        assert!(err.message.contains('@'));
        assert_eq!(err.span.line, 1);
    }
}
