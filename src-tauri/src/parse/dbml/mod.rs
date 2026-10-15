//! Importador de DBML, la sintaxis de dbdiagram.io.
//!
//! Cubre lo que se escribe a diario: `Table` con sus columnas y ajustes,
//! `Ref` sueltas e inline, `Enum`, `TableGroup`, `Project` y notas. Lo que no
//! entiende no lo adivina: lo salta y deja un diagnostico con linea y columna.
//!
//! El documento sale **sin geometria**. Colocar las tablas es cosa del
//! auto-layout, que vive en el frontend porque necesita medir el texto.

mod lexer;

use std::collections::HashMap;

use lexer::{Lexer, Token, TokenKind};

use crate::model::{
    Diagnostic, DiagramKind, Document, Edge, EdgeKind, ErColumn, ErTable, Node, NodeShape, Routing,
    Span,
};

use super::ImportResult;

pub fn import(source: &str, now_ms: i64) -> ImportResult {
    let mut document = Document::new(uuid::Uuid::new_v4().to_string(), DiagramKind::Er, now_ms);

    let tokens = match Lexer::new(source).tokenize() {
        Ok(tokens) => tokens,
        Err(error) => {
            return ImportResult::new(
                document,
                vec![Diagnostic::error("lexError", error.message).at_span(error.span)],
            );
        }
    };

    let mut parser = Parser::new(tokens);
    parser.parse_document();
    parser.finish(&mut document);

    ImportResult::new(document, parser.diagnostics)
}

/// Una relacion pendiente de resolver a nodos y columnas concretos.
#[derive(Debug, Clone)]
struct PendingRef {
    name: Option<String>,
    from: ColumnPath,
    to: ColumnPath,
    /// `>` muchos-a-uno, `<` uno-a-muchos, `-` uno-a-uno, `<>` muchos-a-muchos.
    op: String,
    span: Span,
}

/// `esquema.tabla.columna`, con el esquema opcional.
#[derive(Debug, Clone, PartialEq)]
struct ColumnPath {
    table: String,
    column: String,
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diagnostics: Vec<Diagnostic>,

    tables: Vec<Node>,
    /// Nombre o alias en minusculas -> indice en `tables`.
    by_name: HashMap<String, usize>,
    refs: Vec<PendingRef>,
    groups: Vec<(String, Vec<String>, Span)>,
    project_name: Option<String>,
    project_note: Option<String>,
    /// Enums declarados; sus valores no se dibujan todavia, pero saber que
    /// existen evita avisar de un tipo "desconocido" que si esta definido.
    enums: Vec<String>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            diagnostics: Vec::new(),
            tables: Vec::new(),
            by_name: HashMap::new(),
            refs: Vec::new(),
            groups: Vec::new(),
            project_name: None,
            project_note: None,
            enums: Vec::new(),
        }
    }

    /* --- Navegacion por los tokens ------------------------------------- */

    fn peek(&self) -> &Token {
        // Siempre hay un `End` al final, asi que el indice nunca se sale.
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("el lexer siempre emite al menos End")
        })
    }

    fn at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::End)
    }

    fn bump(&mut self) -> Token {
        let token = self.peek().clone();
        if !self.at_end() {
            self.pos += 1;
        }
        token
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek().kind, TokenKind::Newline) {
            self.pos += 1;
        }
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if self.peek().is(kind) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn error(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::error(code, message).at_span(span));
    }

    fn warn(&mut self, code: &str, message: impl Into<String>, span: Span) {
        self.diagnostics
            .push(Diagnostic::warning(code, message).at_span(span));
    }

    /// Se come un bloque `{ ... }` completo, contando anidamiento. Es como se
    /// recupera el parser de algo que no entiende sin perder el hilo.
    fn skip_block(&mut self) {
        self.skip_newlines();
        if !self.eat(&TokenKind::LBrace) {
            // No era un bloque: descarta hasta el final de la linea.
            while !self.at_end() && !matches!(self.peek().kind, TokenKind::Newline) {
                self.pos += 1;
            }
            return;
        }

        let mut depth = 1;
        while depth > 0 && !self.at_end() {
            match self.bump().kind {
                TokenKind::LBrace => depth += 1,
                TokenKind::RBrace => depth -= 1,
                _ => {}
            }
        }
    }

    /* --- Gramatica ------------------------------------------------------ */

    fn parse_document(&mut self) {
        loop {
            self.skip_newlines();
            if self.at_end() {
                return;
            }

            let token = self.peek().clone();

            if token.is_keyword("table") {
                self.parse_table();
            } else if token.is_keyword("ref") {
                self.parse_standalone_ref();
            } else if token.is_keyword("enum") {
                self.parse_enum();
            } else if token.is_keyword("tablegroup") {
                self.parse_table_group();
            } else if token.is_keyword("project") {
                self.parse_project();
            } else if token.is_keyword("note") {
                // Nota suelta a nivel de documento.
                self.pos += 1;
                self.eat(&TokenKind::Colon);
                self.skip_block();
            } else {
                self.error(
                    "unexpectedToken",
                    format!(
                        "No se esperaba `{}` aqui. DBML espera Table, Ref, Enum, TableGroup, Project o Note.",
                        describe(&token)
                    ),
                    token.span,
                );
                // Saltar hasta la siguiente linea para no repetir el error en
                // cada token de la misma construccion.
                while !self.at_end() && !matches!(self.peek().kind, TokenKind::Newline) {
                    self.pos += 1;
                }
            }
        }
    }

    /// `Table [esquema.]nombre [as alias] { columnas }`
    fn parse_table(&mut self) {
        let keyword = self.bump();

        let Some((schema, name, span)) = self.parse_qualified_name() else {
            self.error(
                "missingTableName",
                "Falta el nombre de la tabla despues de `Table`.",
                keyword.span,
            );
            self.skip_block();
            return;
        };

        let mut alias = None;
        if self.peek().is_keyword("as") {
            self.pos += 1;
            match self.bump().name().map(str::to_string) {
                Some(a) => alias = Some(a),
                None => self.error("missingAlias", "Falta el alias despues de `as`.", span),
            }
        }

        let mut table = ErTable {
            schema,
            alias: alias.clone(),
            ..Default::default()
        };

        // Ajustes de la propia tabla: [headercolor: #3498db, note: '...']
        let mut header_color = None;
        if self.peek().is(&TokenKind::LBracket) {
            for (key, value, _) in self.parse_settings() {
                match key.as_str() {
                    "note" => table.note = Some(value),
                    "headercolor" => header_color = Some(value),
                    _ => {}
                }
            }
        }

        self.skip_newlines();
        if !self.eat(&TokenKind::LBrace) {
            self.error(
                "missingBrace",
                format!("Falta `{{` para abrir el cuerpo de la tabla `{name}`."),
                span,
            );
            return;
        }

        let node_id = format!("t_{}", slug(&name));
        let mut seen_columns: HashMap<String, Span> = HashMap::new();

        loop {
            self.skip_newlines();
            if self.at_end() {
                self.error(
                    "unterminatedTable",
                    format!("La tabla `{name}` se quedo sin cerrar con `}}`."),
                    span,
                );
                break;
            }
            if self.eat(&TokenKind::RBrace) {
                break;
            }

            let token = self.peek().clone();

            if token.is_keyword("note") {
                self.pos += 1;
                self.eat(&TokenKind::Colon);
                if let Some(text) = self.bump().name().map(str::to_string) {
                    table.note = Some(text);
                }
                continue;
            }

            if token.is_keyword("indexes") {
                self.pos += 1;
                // Los indices no se dibujan; conviene decirlo en vez de
                // tragarselos en silencio.
                self.warn(
                    "ignoredIndexes",
                    format!(
                        "El bloque `indexes` de `{name}` se importa sin efecto: los indices no se dibujan."
                    ),
                    token.span,
                );
                self.skip_block();
                continue;
            }

            match self.parse_column(&node_id, &name) {
                Some(column) => {
                    if let Some(previous) = seen_columns.get(&column.name.to_lowercase()) {
                        self.error(
                            "duplicateColumn",
                            format!(
                                "La tabla `{name}` ya tiene una columna `{}` (linea {}).",
                                column.name, previous.line
                            ),
                            token.span,
                        );
                    } else {
                        seen_columns.insert(column.name.to_lowercase(), token.span);
                        table.columns.push(column);
                    }
                }
                None => {
                    // `parse_column` ya reporto; avanzar para no atascarse.
                    while !self.at_end()
                        && !matches!(self.peek().kind, TokenKind::Newline | TokenKind::RBrace)
                    {
                        self.pos += 1;
                    }
                }
            }
        }

        let key = name.to_lowercase();
        if let Some(&existing) = self.by_name.get(&key) {
            let _ = existing;
            self.error(
                "duplicateTable",
                format!("La tabla `{name}` esta declarada mas de una vez."),
                span,
            );
            return;
        }

        let mut node = Node::table(&node_id, &name, table);
        if let Some(color) = header_color {
            node.style.fill = Some(color);
        }

        let index = self.tables.len();
        self.by_name.insert(key, index);
        if let Some(alias) = alias {
            self.by_name.insert(alias.to_lowercase(), index);
        }
        self.tables.push(node);
    }

    /// Una linea de columna: `nombre tipo [ajustes]`.
    fn parse_column(&mut self, node_id: &str, table_name: &str) -> Option<ErColumn> {
        let first = self.peek().clone();
        let name = first.name()?.to_string();
        self.pos += 1;

        let column_id = format!("{node_id}.{}", slug(&name));
        let mut column = ErColumn::new(&column_id, &name);

        // Tipo: palabra, con `(n)` o `[]` opcionales.
        if let Some(type_name) = self.peek().name().map(str::to_string) {
            self.pos += 1;
            let mut data_type = type_name;

            if self.eat(&TokenKind::LParen) {
                let mut args = Vec::new();
                loop {
                    match &self.peek().kind {
                        TokenKind::RParen | TokenKind::End => break,
                        TokenKind::Comma => {
                            self.pos += 1;
                            args.push(",".to_string());
                        }
                        TokenKind::Number(n) => {
                            args.push(n.clone());
                            self.pos += 1;
                        }
                        TokenKind::Word(w) => {
                            args.push(w.clone());
                            self.pos += 1;
                        }
                        _ => {
                            self.pos += 1;
                        }
                    }
                }
                self.eat(&TokenKind::RParen);
                data_type = format!("{data_type}({})", args.join("").replace(',', ", "));
            }

            if self.eat(&TokenKind::LBracket) && self.eat(&TokenKind::RBracket) {
                data_type.push_str("[]");
            } else {
                // No era `[]`: era el bloque de ajustes. Retroceder.
                if matches!(
                    self.tokens.get(self.pos.saturating_sub(1)).map(|t| &t.kind),
                    Some(TokenKind::LBracket)
                ) {
                    self.pos -= 1;
                }
            }

            column.data_type = data_type;
        }

        if self.peek().is(&TokenKind::LBracket) {
            for (key, value, span) in self.parse_settings() {
                self.apply_column_setting(&mut column, &key, &value, span, table_name, &name);
            }
        }

        Some(column)
    }

    fn apply_column_setting(
        &mut self,
        column: &mut ErColumn,
        key: &str,
        value: &str,
        span: Span,
        table_name: &str,
        column_name: &str,
    ) {
        match key {
            "pk" | "primary key" => {
                column.primary_key = true;
                column.nullable = false;
            }
            "not null" => column.nullable = false,
            "null" => column.nullable = true,
            "unique" => column.unique = true,
            "increment" => column.increment = true,
            "default" => column.default_value = Some(value.to_string()),
            "note" => column.note = Some(value.to_string()),
            "ref" => {
                // `ref: > otra.columna` declarada dentro de la columna.
                let Some((op, path)) = split_inline_ref(value) else {
                    self.error(
                        "badInlineRef",
                        format!("No se entiende la relacion `{value}`. Se esperaba algo como `> tabla.columna`."),
                        span,
                    );
                    return;
                };
                self.refs.push(PendingRef {
                    name: None,
                    from: ColumnPath {
                        table: table_name.to_string(),
                        column: column_name.to_string(),
                    },
                    to: path,
                    op,
                    span,
                });
            }
            other => {
                self.warn(
                    "unknownSetting",
                    format!("Ajuste `{other}` no soportado; se ignora."),
                    span,
                );
            }
        }
    }

    /// `[clave: valor, bandera, ...]`. Devuelve (clave, valor, posicion).
    fn parse_settings(&mut self) -> Vec<(String, String, Span)> {
        let mut out = Vec::new();
        if !self.eat(&TokenKind::LBracket) {
            return out;
        }

        loop {
            match &self.peek().kind {
                TokenKind::RBracket => {
                    self.pos += 1;
                    break;
                }
                TokenKind::End => break,
                TokenKind::Comma | TokenKind::Newline => {
                    self.pos += 1;
                }
                _ => {
                    let span = self.peek().span;

                    // La clave puede ser de varias palabras: `primary key`.
                    let mut key = String::new();
                    while let TokenKind::Word(word) = &self.peek().kind {
                        if !key.is_empty() {
                            key.push(' ');
                        }
                        key.push_str(&word.to_lowercase());
                        self.pos += 1;
                    }

                    if key.is_empty() {
                        self.pos += 1;
                        continue;
                    }

                    let mut value = String::new();
                    if self.eat(&TokenKind::Colon) {
                        value = self.read_setting_value();
                    }

                    out.push((key, value, span));
                }
            }
        }

        out
    }

    /// Lee el valor de un ajuste hasta donde termina.
    ///
    /// Para en el salto de linea y en `}` ademas de en la coma y el corchete:
    /// dentro de un `Project`, los ajustes van uno por linea y sin corchetes,
    /// y sin ese corte el primero se tragaba el resto del documento. Las notas
    /// multilinea no sufren, porque el lexer ya las entrega como un solo
    /// token de texto.
    fn read_setting_value(&mut self) -> String {
        let mut parts: Vec<String> = Vec::new();

        loop {
            match &self.peek().kind {
                TokenKind::Comma
                | TokenKind::RBracket
                | TokenKind::RBrace
                | TokenKind::Newline
                | TokenKind::End => break,
                TokenKind::Text(t) => {
                    parts.push(t.clone());
                    self.pos += 1;
                }
                TokenKind::Expression(e) => {
                    parts.push(format!("`{e}`"));
                    self.pos += 1;
                }
                TokenKind::Word(w) => {
                    parts.push(w.clone());
                    self.pos += 1;
                }
                TokenKind::Number(n) => {
                    parts.push(n.clone());
                    self.pos += 1;
                }
                TokenKind::Op(o) => {
                    parts.push(o.clone());
                    self.pos += 1;
                }
                TokenKind::Dot => {
                    parts.push(".".into());
                    self.pos += 1;
                }
                _ => {
                    self.pos += 1;
                }
            }
        }

        parts.join(" ").replace(" . ", ".")
    }

    /// `Ref [nombre]: a.col > b.col` o `Ref [nombre] { a.col > b.col }`
    fn parse_standalone_ref(&mut self) {
        let keyword = self.bump();

        // `Ref nombre: ...` lleva nombre; `Ref: ...` no.
        let mut name = None;
        if let Some(word) = self.peek().name().map(str::to_string)
            && !self.peek().is(&TokenKind::Colon)
        {
            name = Some(word);
            self.pos += 1;
        }

        let braced = {
            self.skip_newlines();
            if self.peek().is(&TokenKind::LBrace) {
                self.pos += 1;
                true
            } else {
                self.eat(&TokenKind::Colon);
                false
            }
        };

        loop {
            self.skip_newlines();
            if braced && self.eat(&TokenKind::RBrace) {
                return;
            }
            if self.at_end() {
                if braced {
                    self.error(
                        "unterminatedRef",
                        "El bloque `Ref` se quedo sin cerrar con `}`.",
                        keyword.span,
                    );
                }
                return;
            }

            let span = self.peek().span;
            let Some(from) = self.parse_column_path() else {
                self.error(
                    "badRef",
                    "Se esperaba `tabla.columna` al principio de la relacion.",
                    span,
                );
                self.skip_to_line_end();
                if !braced {
                    return;
                }
                continue;
            };

            let TokenKind::Op(op) = self.peek().kind.clone() else {
                self.error(
                    "badRef",
                    "Falta el operador de la relacion: `>`, `<`, `-` o `<>`.",
                    self.peek().span,
                );
                self.skip_to_line_end();
                if !braced {
                    return;
                }
                continue;
            };
            self.pos += 1;

            let Some(to) = self.parse_column_path() else {
                self.error(
                    "badRef",
                    "Se esperaba `tabla.columna` despues del operador.",
                    self.peek().span,
                );
                self.skip_to_line_end();
                if !braced {
                    return;
                }
                continue;
            };

            // Ajustes de la relacion (`[delete: cascade]`): no afectan al
            // dibujo, se leen para no dejarlos sueltos en el flujo.
            if self.peek().is(&TokenKind::LBracket) {
                self.parse_settings();
            }

            self.refs.push(PendingRef {
                name: name.clone(),
                from,
                to,
                op,
                span,
            });

            if !braced {
                return;
            }
        }
    }

    fn skip_to_line_end(&mut self) {
        while !self.at_end() && !matches!(self.peek().kind, TokenKind::Newline | TokenKind::RBrace)
        {
            self.pos += 1;
        }
    }

    /// `[esquema.]tabla.columna`
    fn parse_column_path(&mut self) -> Option<ColumnPath> {
        let mut parts = Vec::new();

        let first = self.peek().name()?.to_string();
        self.pos += 1;
        parts.push(first);

        while self.peek().is(&TokenKind::Dot) {
            self.pos += 1;
            let next = self.peek().name()?.to_string();
            self.pos += 1;
            parts.push(next);
        }

        match parts.len() {
            // `tabla.columna`
            2 => Some(ColumnPath {
                table: parts[0].clone(),
                column: parts[1].clone(),
            }),
            // `esquema.tabla.columna`: el esquema no cambia a que tabla apunta.
            n if n >= 3 => Some(ColumnPath {
                table: parts[n - 2].clone(),
                column: parts[n - 1].clone(),
            }),
            _ => None,
        }
    }

    /// `[esquema.]nombre`
    fn parse_qualified_name(&mut self) -> Option<(Option<String>, String, Span)> {
        let token = self.peek().clone();
        let first = token.name()?.to_string();
        self.pos += 1;

        if self.peek().is(&TokenKind::Dot) {
            self.pos += 1;
            let name = self.peek().name()?.to_string();
            self.pos += 1;
            return Some((Some(first), name, token.span));
        }

        Some((None, first, token.span))
    }

    fn parse_enum(&mut self) {
        self.pos += 1;
        if let Some((_, name, _)) = self.parse_qualified_name() {
            self.enums.push(name);
        }
        self.skip_block();
    }

    /// `TableGroup nombre { tabla1 tabla2 }`
    fn parse_table_group(&mut self) {
        let keyword = self.bump();

        let Some((_, name, span)) = self.parse_qualified_name() else {
            self.error(
                "missingGroupName",
                "Falta el nombre del grupo despues de `TableGroup`.",
                keyword.span,
            );
            self.skip_block();
            return;
        };

        if self.peek().is(&TokenKind::LBracket) {
            self.parse_settings();
        }

        self.skip_newlines();
        if !self.eat(&TokenKind::LBrace) {
            self.error(
                "missingBrace",
                format!("Falta `{{` para abrir el grupo `{name}`."),
                span,
            );
            return;
        }

        let mut members = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_end() || self.eat(&TokenKind::RBrace) {
                break;
            }
            match self.parse_qualified_name() {
                Some((_, member, _)) => members.push(member),
                None => {
                    self.pos += 1;
                }
            }
        }

        self.groups.push((name, members, span));
    }

    fn parse_project(&mut self) {
        self.pos += 1;
        if let Some((_, name, _)) = self.parse_qualified_name() {
            self.project_name = Some(name);
        }

        self.skip_newlines();
        if !self.eat(&TokenKind::LBrace) {
            return;
        }

        loop {
            self.skip_newlines();
            if self.at_end() || self.eat(&TokenKind::RBrace) {
                break;
            }

            let is_note = self.peek().is_keyword("note");
            let key_present = self.peek().name().is_some();
            if key_present {
                self.pos += 1;
                self.eat(&TokenKind::Colon);
                let value = self.read_setting_value();
                if is_note {
                    self.project_note = Some(value);
                }
            } else {
                self.pos += 1;
            }
        }
    }

    /* --- Resolucion ----------------------------------------------------- */

    /// Convierte lo acumulado en el documento final.
    fn finish(&mut self, document: &mut Document) {
        if let Some(name) = self.project_name.take() {
            document.meta.title = name;
        }
        if let Some(note) = self.project_note.take() {
            document.meta.description = Some(note);
        }

        self.resolve_groups();
        document.nodes = std::mem::take(&mut self.tables);
        self.resolve_refs(document);
    }

    /// Los `TableGroup` se convierten en paquetes contenedores.
    fn resolve_groups(&mut self) {
        let groups = std::mem::take(&mut self.groups);

        for (name, members, span) in groups {
            let group_id = format!("g_{}", slug(&name));
            let mut assigned = 0;

            for member in &members {
                match self.by_name.get(&member.to_lowercase()) {
                    Some(&index) => {
                        self.tables[index].parent = Some(group_id.clone());
                        assigned += 1;
                    }
                    None => self.warn(
                        "unknownGroupMember",
                        format!("El grupo `{name}` menciona la tabla `{member}`, que no existe."),
                        span,
                    ),
                }
            }

            if assigned == 0 {
                continue;
            }

            // Los contenedores van primero: el lienzo y el exportador esperan
            // ver al padre antes que a sus hijos.
            let group = Node::new(&group_id, &name, NodeShape::Package);
            self.tables.insert(0, group);
            for index in self.by_name.values_mut() {
                *index += 1;
            }
        }
    }

    fn resolve_refs(&mut self, document: &mut Document) {
        let refs = std::mem::take(&mut self.refs);

        for (index, pending) in refs.into_iter().enumerate() {
            let Some(source) = self.locate(&pending.from, &pending.span, document) else {
                continue;
            };
            let Some(target) = self.locate(&pending.to, &pending.span, document) else {
                continue;
            };

            // El extremo "muchos" es el que lleva la clave ajena.
            let (from_card, to_card) = match pending.op.as_str() {
                ">" => ("*", "1"),
                "<" => ("1", "*"),
                "-" => ("1", "1"),
                "<>" => ("*", "*"),
                other => {
                    self.error(
                        "badRefOperator",
                        format!("Operador de relacion `{other}` desconocido."),
                        pending.span,
                    );
                    continue;
                }
            };

            // Marcar como clave ajena el lado "muchos".
            let fk_target = if from_card == "*" {
                Some(&source)
            } else if to_card == "*" {
                Some(&target)
            } else {
                None
            };
            if let Some(location) = fk_target
                && let Some(node) = document.node_mut(&location.node)
                && let Some(table) = node.data.as_er_mut()
                && let Some(column) = table.column_mut(&location.column)
            {
                column.foreign_key = true;
            }

            let id = pending
                .name
                .map(|n| format!("r_{}", slug(&n)))
                .unwrap_or_else(|| format!("r_{index}"));

            let mut edge = Edge::new(id, &source.node, &target.node, EdgeKind::Association);
            edge.source.port = Some(source.column.clone());
            edge.target.port = Some(target.column.clone());
            edge.source.label = Some(from_card.to_string());
            edge.target.label = Some(to_card.to_string());
            edge.routing = Routing::Orthogonal;

            document.edges.push(edge);
        }
    }

    /// Resuelve `tabla.columna` a ids concretos.
    fn locate(&mut self, path: &ColumnPath, span: &Span, document: &Document) -> Option<Location> {
        let Some(&index) = self.by_name.get(&path.table.to_lowercase()) else {
            self.diagnostics.push(
                Diagnostic::error(
                    "unknownTable",
                    format!(
                        "La relacion apunta a la tabla `{}`, que no existe.",
                        path.table
                    ),
                )
                .at_span(*span),
            );
            return None;
        };

        let node = document.nodes.get(index)?;
        let table = node.er()?;

        let Some(column) = table.column_by_name(&path.column) else {
            self.diagnostics.push(
                Diagnostic::error(
                    "unknownColumn",
                    format!(
                        "La tabla `{}` no tiene ninguna columna `{}`.",
                        path.table, path.column
                    ),
                )
                .at_span(*span),
            );
            return None;
        };

        Some(Location {
            node: node.id.clone(),
            column: column.id.clone(),
        })
    }
}

struct Location {
    node: String,
    column: String,
}

/// Convierte un nombre en algo usable como id: minusculas y sin espacios.
fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Parte `> tabla.columna` en su operador y su destino.
fn split_inline_ref(value: &str) -> Option<(String, ColumnPath)> {
    let trimmed = value.trim();

    // `<>` antes que `<`, o el primero nunca se reconoceria.
    let (op, rest) = ["<>", ">", "<", "-"]
        .into_iter()
        .find_map(|op| trimmed.strip_prefix(op).map(|rest| (op, rest)))?;

    let parts: Vec<&str> = rest.trim().split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    Some((
        op.to_string(),
        ColumnPath {
            table: parts[parts.len() - 2].trim().to_string(),
            column: parts[parts.len() - 1].trim().to_string(),
        },
    ))
}

/// Como describir un token en un mensaje de error.
fn describe(token: &Token) -> String {
    match &token.kind {
        TokenKind::Word(w) | TokenKind::Text(w) | TokenKind::Number(w) => w.clone(),
        TokenKind::Expression(e) => format!("`{e}`"),
        TokenKind::LBrace => "{".into(),
        TokenKind::RBrace => "}".into(),
        TokenKind::LBracket => "[".into(),
        TokenKind::RBracket => "]".into(),
        TokenKind::LParen => "(".into(),
        TokenKind::RParen => ")".into(),
        TokenKind::Comma => ",".into(),
        TokenKind::Colon => ":".into(),
        TokenKind::Dot => ".".into(),
        TokenKind::Op(o) => o.clone(),
        TokenKind::Newline => "fin de linea".into(),
        TokenKind::End => "fin del documento".into(),
    }
}

#[cfg(test)]
mod tests;
