//! Generador de diagramas de actividad de PlantUML.
//!
//! Es el camino inverso del importador, y no es el mismo trabajo al reves.
//! El importador lee una lista de sentencias y va dejando flechas; aqui hay
//! un grafo —que ademas el usuario ha podido editar a mano— y hay que
//! recuperar de el las estructuras que lo escribieron: donde empieza un `if`,
//! donde vuelven a juntarse sus dos ramas, cual de los dos bucles es.
//!
//! El recorrido se apoya en tres cosas que se calculan antes de escribir
//! nada:
//!
//! - **Las vueltas atras.** Un recorrido en profundidad marca las aristas que
//!   apuntan a un nodo que todavia esta abierto en el camino. Sin ellas el
//!   grafo no tiene ciclos y todo lo demas se puede preguntar.
//! - **Lo alcanzable** desde un nodo, sin contar esas vueltas.
//! - **El punto de reunion** de varias salidas: el primer nodo al que llegan
//!   todas. Ahi es donde cierra el `if`, el `fork` o el `split`.
//!
//! Los dos bucles se distinguen por donde esta la condicion. En un `while` el
//! rombo es la entrada del bucle —la vuelta atras apunta a el— y tiene una
//! salida que se va fuera. En un `repeat` el rombo esta al final y es la
//! vuelta atras la que sale de el.
//!
//! Hay una cosa que el texto generado no recupera, y no por descuido: donde
//! cerraba un `if` cuya otra rama acaba en `stop`. Si una rama se corta, las
//! dos no vuelven a juntarse en ningun sitio, y el grafo ya no guarda en que
//! punto decidio el autor cerrar el bloque. Todo lo que seguia se escribe
//! entonces dentro de la rama que sigue viva. Es el mismo flujo —la ida y
//! vuelta lo comprueba— pero con mas sangria que el original.
//!
//! Y un grafo dibujado a mano puede directamente no ser expresable: PlantUML
//! no tiene como escribir un salto al centro de un bloque. Antes de cerrar,
//! el generador compara lo que ha escrito con las aristas del documento y
//! nombra una a una las que no han cabido. Inventarse un flujo parecido y
//! callarse seria el peor fallo posible de este fichero.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Write as _;

use crate::model::{Document, Edge, Node, NodeShape};

/// Sangria de cada nivel.
const INDENT: &str = "  ";

pub fn emit(doc: &Document) -> String {
    let mut out = String::from("@startuml\n");

    if doc.meta.title != doc.kind.title() {
        let _ = writeln!(out, "title {}", doc.meta.title.replace('\n', " "));
    }
    out.push('\n');

    Writer::new(doc).run(&mut out);

    out.push_str("@enduml\n");
    out
}

struct Writer<'a> {
    doc: &'a Document,
    /// Aristas que salen de cada nodo, en el orden en que estan guardadas.
    out_edges: HashMap<&'a str, Vec<&'a Edge>>,
    /// Ids de las aristas que cierran un ciclo.
    back: HashSet<&'a str>,
    /// Nodos ya escritos.
    done: HashSet<&'a str>,
    /// Bucles abiertos, para no volver a entrar en ellos al recorrerlos.
    loops: HashSet<&'a str>,
    /// Aristas que el texto generado si expresa.
    used: HashSet<&'a str>,
    /// Particiones abiertas, de fuera a dentro.
    open: Vec<&'a str>,
}

impl<'a> Writer<'a> {
    fn new(doc: &'a Document) -> Self {
        let mut out_edges: HashMap<&'a str, Vec<&'a Edge>> = HashMap::new();
        for edge in &doc.edges {
            out_edges
                .entry(edge.source.node.as_str())
                .or_default()
                .push(edge);
        }

        let back = back_edges(doc, &out_edges);

        Writer {
            doc,
            out_edges,
            back,
            done: HashSet::new(),
            loops: HashSet::new(),
            used: HashSet::new(),
            open: Vec::new(),
        }
    }

    /* --- Consultas sobre el grafo ------------------------------------- */

    fn node_of(&self, id: &str) -> Option<&'a Node> {
        self.doc.node(id)
    }

    /// Aristas que salen, sin las vueltas atras.
    fn forward(&self, id: &str) -> Vec<&'a Edge> {
        self.out_edges
            .get(id)
            .map(|edges| {
                edges
                    .iter()
                    .copied()
                    .filter(|e| !self.back.contains(e.id.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// La vuelta atras que sale de un nodo, si la hay.
    fn back_out(&self, id: &str) -> Option<&'a Edge> {
        self.out_edges
            .get(id)?
            .iter()
            .copied()
            .find(|e| self.back.contains(e.id.as_str()))
    }

    /// Las vueltas atras que entran a un nodo.
    fn back_in(&self, id: &str) -> Vec<&'a Edge> {
        self.doc
            .edges
            .iter()
            .filter(|e| self.back.contains(e.id.as_str()) && e.target.node == id)
            .collect()
    }

    /// Todo lo que se alcanza desde un nodo sin dar vueltas atras.
    fn reach(&self, from: &'a str) -> HashSet<&'a str> {
        let mut seen = HashSet::new();
        let mut stack = vec![from];

        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            for edge in self.forward(id) {
                stack.push(edge.target.node.as_str());
            }
        }

        seen
    }

    /// Primer nodo al que llegan todas las salidas: donde vuelven a juntarse.
    ///
    /// Se busca en anchura desde la primera, para quedarse con el mas cercano
    /// y no con uno de mas abajo que tambien valdria.
    fn reunion(&self, exits: &[&'a str]) -> Option<&'a str> {
        let reaches: Vec<HashSet<&'a str>> = exits.iter().map(|id| self.reach(id)).collect();

        let mut queue = VecDeque::from([*exits.first()?]);
        let mut seen = HashSet::new();

        while let Some(id) = queue.pop_front() {
            if !seen.insert(id) {
                continue;
            }
            if reaches.iter().all(|r| r.contains(id)) {
                return Some(id);
            }
            for edge in self.forward(id) {
                queue.push_back(edge.target.node.as_str());
            }
        }

        None
    }

    /* --- Escritura ----------------------------------------------------- */

    /// Apunta que esta arista ha quedado escrita.
    fn mark(&mut self, edge: &'a Edge) {
        self.used.insert(edge.id.as_str());
    }

    fn indent(&self, depth: usize) -> String {
        INDENT.repeat(depth + self.open.len())
    }

    fn run(&mut self, out: &mut String) {
        // Un diagrama puede tener trozos sueltos: lo que va detras de un
        // `stop` no cuelga de nada. Cada uno se escribe por su cuenta, que es
        // justo como se habia escrito.
        while let Some(entry) = self.next_entry() {
            self.region(Some(entry), None, 0, out);
        }

        self.report(out);
    }

    /// Dice que pasos del diagrama no han cabido en el texto.
    ///
    /// Un grafo dibujado a mano puede no ser expresable —PlantUML no tiene
    /// como escribir un salto al centro de un bloque— y la forma peor de
    /// resolverlo seria escribir un flujo parecido y callarse. Aqui se
    /// nombran uno a uno.
    fn report(&mut self, out: &mut String) {
        let pendientes: Vec<String> = self
            .doc
            .edges
            .iter()
            .filter(|e| !self.used.contains(e.id.as_str()))
            .map(|e| {
                format!(
                    "{} a {}",
                    self.caption(&e.source.node),
                    self.caption(&e.target.node)
                )
            })
            .collect();

        for paso in pendientes {
            let _ = writeln!(out, "' sin escribir: el paso de {paso}");
        }
    }

    /// Por donde empezar el siguiente trozo: el arranque si queda alguno, y
    /// si no un nodo al que no llegue nada que este sin escribir.
    fn next_entry(&self) -> Option<&'a str> {
        let pending = |node: &&'a Node| {
            node.shape != NodeShape::Package && !self.done.contains(node.id.as_str())
        };

        let sueltos: Vec<&'a Node> = self.doc.nodes.iter().filter(pending).collect();

        sueltos
            .iter()
            .find(|n| n.shape == NodeShape::InitialNode)
            .or_else(|| {
                sueltos.iter().find(|n| {
                    !self.doc.edges.iter().any(|e| {
                        e.target.node == n.id && !self.done.contains(e.source.node.as_str())
                    })
                })
            })
            .or(sueltos.first())
            .map(|n| n.id.as_str())
    }

    /// Escribe el camino que va de `entry` hasta `stop`, sin incluirlo.
    fn region(
        &mut self,
        entry: Option<&'a str>,
        stop: Option<&'a str>,
        depth: usize,
        out: &mut String,
    ) {
        let opened = self.open.len();
        let mut current = entry;

        while let Some(id) = current {
            if Some(id) == stop {
                break;
            }
            if self.done.contains(id) {
                // Ya escrito en otro sitio: es un salto, y PlantUML no tiene
                // donde ponerlo.
                let name = self.caption(id);
                let indent = self.indent(depth);
                let _ = writeln!(
                    out,
                    "{indent}' salto a `{name}`, que no se puede escribir aqui"
                );
                break;
            }

            current = self.step(id, stop, depth, out);
        }

        self.close_partitions(opened, depth, out);
    }

    /// Escribe lo que haya en un nodo y devuelve por donde sigue.
    fn step(
        &mut self,
        id: &'a str,
        stop: Option<&'a str>,
        depth: usize,
        out: &mut String,
    ) -> Option<&'a str> {
        let node = self.node_of(id)?;

        if node.shape == NodeShape::Package {
            return self.forward(id).first().map(|e| e.target.node.as_str());
        }

        // La particion va antes que el bucle: si no, un `repeat` que vive
        // dentro de una se escribiria fuera y sus nodos cambiarian de caja.
        self.enter_partition(node.parent.as_deref(), depth, out);

        if let Some(next) = self.loop_here(id, node, depth, out) {
            return next;
        }

        match node.shape {
            NodeShape::Diamond => self.decision(id, stop, depth, out),
            NodeShape::Bar => self.fork(id, depth, out),
            _ => {
                self.done.insert(id);
                let indent = self.indent(depth);
                let _ = writeln!(out, "{indent}{}", step_text(node, &indent));

                let next = self.forward(id);
                match next.len() {
                    0 => None,
                    1 => {
                        self.mark(next[0]);
                        Some(next[0].target.node.as_str())
                    }
                    // Varias salidas de algo que no es un rombo: se reparte
                    // sin preguntar, que es lo que `split` significa.
                    _ => self.split(id, depth, out),
                }
            }
        }
    }

    /* --- Bucles --------------------------------------------------------- */

    /// Si en este nodo empieza un bucle, lo escribe entero y devuelve por
    /// donde sigue. `None` si aqui no empieza ninguno.
    #[allow(clippy::option_option)]
    fn loop_here(
        &mut self,
        id: &'a str,
        node: &'a Node,
        depth: usize,
        out: &mut String,
    ) -> Option<Option<&'a str>> {
        if self.loops.contains(id) {
            return None;
        }

        if node.shape == NodeShape::Diamond
            && let Some((body, exit)) = self.while_parts(id)
        {
            return Some(self.while_loop(id, node, body, exit, depth, out));
        }

        // La vuelta atras llega de un rombo que esta debajo: la condicion va
        // al final, asi que el cuerpo corre al menos una vez.
        let tail = self
            .back_in(id)
            .into_iter()
            .find(|e| {
                self.node_of(&e.source.node)
                    .is_some_and(|n| n.shape == NodeShape::Diamond)
            })
            .map(|e| e.source.node.as_str())?;

        Some(self.repeat_loop(id, tail, depth, out))
    }

    /// Las dos salidas de un rombo que manda un `while`: la que entra al
    /// cuerpo y la que se va fuera. `None` si este rombo no manda ninguno.
    ///
    /// Un rombo manda un `while` cuando una vuelta atras apunta a el —la
    /// condicion se pregunta antes de cada pasada— y ademas le queda una
    /// salida que abandona el bucle. Esa segunda parte es la que importa: el
    /// rombo que esta *dentro* de un `repeat` tambien recibe la vuelta atras,
    /// pero todas sus salidas se quedan en el ciclo.
    ///
    /// Abandonar el bucle no siempre es ir hacia delante. En un `while`
    /// metido dentro de otro, la salida del de dentro es justo la vuelta
    /// atras del de fuera.
    fn while_parts(&self, id: &'a str) -> Option<(&'a Edge, &'a Edge)> {
        let sources: Vec<&'a str> = self
            .back_in(id)
            .iter()
            .map(|e| e.source.node.as_str())
            .collect();
        if sources.is_empty() {
            return None;
        }

        let returns = |target: &'a str| {
            let reach = self.reach(target);
            sources.iter().any(|source| reach.contains(source))
        };

        let all = self.out_edges.get(id)?;

        let body = all
            .iter()
            .copied()
            .find(|e| !self.back.contains(e.id.as_str()) && returns(e.target.node.as_str()))?;

        let exit = all.iter().copied().find(|e| {
            e.id != body.id
                && (self.back.contains(e.id.as_str()) || !returns(e.target.node.as_str()))
        })?;

        Some((body, exit))
    }

    fn while_loop(
        &mut self,
        id: &'a str,
        node: &'a Node,
        body: &'a Edge,
        exit: &'a Edge,
        depth: usize,
        out: &mut String,
    ) -> Option<&'a str> {
        self.done.insert(id);
        self.loops.insert(id);
        self.mark(body);
        self.mark(exit);
        for edge in self.back_in(id) {
            self.mark(edge);
        }

        let indent = self.indent(depth);
        let _ = writeln!(
            out,
            "{indent}while ({}){}",
            condition(node),
            tail_label("is", body.label.as_deref())
        );

        self.region(Some(body.target.node.as_str()), Some(id), depth + 1, out);

        let _ = writeln!(
            out,
            "{indent}endwhile{}",
            tail_label("", exit.label.as_deref())
        );

        self.loops.remove(id);
        Some(exit.target.node.as_str())
    }

    fn repeat_loop(
        &mut self,
        id: &'a str,
        tail: &'a str,
        depth: usize,
        out: &mut String,
    ) -> Option<&'a str> {
        self.loops.insert(id);

        let indent = self.indent(depth);
        let _ = writeln!(out, "{indent}repeat");

        self.region(Some(id), Some(tail), depth + 1, out);

        self.done.insert(tail);
        let condition = self.node_of(tail).map(condition).unwrap_or_default();

        let back = self.back_out(tail);
        let inner = back.and_then(|e| e.label.clone());
        if let Some(edge) = back {
            self.mark(edge);
        }

        let _ = writeln!(
            out,
            "{indent}repeat while ({condition}){}",
            tail_label("is", inner.as_deref())
        );

        self.loops.remove(id);

        let exit = self.forward(tail);
        let exit = exit.first()?;
        self.mark(exit);
        Some(exit.target.node.as_str())
    }

    /* --- Condiciones ----------------------------------------------------- */

    fn decision(
        &mut self,
        id: &'a str,
        stop: Option<&'a str>,
        depth: usize,
        out: &mut String,
    ) -> Option<&'a str> {
        let edges = self.forward(id);
        if edges.len() < 2 {
            // Un rombo con una sola salida no decide nada; se escribe como un
            // paso mas para no perderlo.
            self.done.insert(id);
            let node = self.node_of(id)?;
            let indent = self.indent(depth);
            let _ = writeln!(out, "{indent}:{};", escape(&node.label));

            let next = edges.first()?;
            self.mark(next);
            return Some(next.target.node.as_str());
        }

        let exits: Vec<&'a str> = edges.iter().map(|e| e.target.node.as_str()).collect();
        let after = self.reunion(&exits).or(stop);

        let indent = self.indent(depth);
        let mut current = id;
        let mut word = "if";

        // `elseif` se recupera igual que se escribio: la salida que no se tomo
        // lleva a otro rombo que cierra donde cierra este.
        loop {
            let edges = self.forward(current);
            let node = self.node_of(current)?;
            self.done.insert(current);

            let then = edges.first()?;
            self.mark(then);
            let _ = writeln!(
                out,
                "{indent}{word} ({}) then{}",
                condition(node),
                tail_label("", then.label.as_deref())
            );
            self.region(Some(then.target.node.as_str()), after, depth + 1, out);

            let Some(otherwise) = edges.get(1) else {
                break;
            };

            if edges.len() > 2 {
                let _ = writeln!(
                    out,
                    "{indent}' el rombo tiene {} salidas y aqui solo caben dos",
                    edges.len()
                );
            }

            self.mark(otherwise);
            let next = otherwise.target.node.as_str();
            if self.chains_as_elseif(next, otherwise, after) {
                current = next;
                word = "elseif";
                continue;
            }

            // Sin `else` escrito, la salida que no se toma es lo que sigue:
            // no hay rama que escribir.
            if Some(next) == after {
                break;
            }

            let _ = writeln!(
                out,
                "{indent}else{}",
                tail_label("", otherwise.label.as_deref())
            );
            self.region(Some(next), after, depth + 1, out);
            break;
        }

        let _ = writeln!(out, "{indent}endif");
        after
    }

    /// `true` si la rama que no se tomo es otro rombo que cierra en el mismo
    /// sitio: eso es un `elseif` y no un `if` anidado dentro del `else`.
    fn chains_as_elseif(&self, next: &'a str, edge: &'a Edge, after: Option<&'a str>) -> bool {
        if edge.label.is_some() || self.done.contains(next) || self.loops.contains(next) {
            return false;
        }
        if self.node_of(next).map(|n| n.shape) != Some(NodeShape::Diamond) {
            return false;
        }
        if self.while_parts(next).is_some() {
            return false;
        }

        let exits: Vec<&'a str> = self
            .forward(next)
            .iter()
            .map(|e| e.target.node.as_str())
            .collect();

        exits.len() >= 2 && self.reunion(&exits) == after
    }

    /* --- Ramas paralelas -------------------------------------------------- */

    fn fork(&mut self, id: &'a str, depth: usize, out: &mut String) -> Option<&'a str> {
        self.done.insert(id);

        let edges = self.forward(id);
        if edges.len() < 2 {
            // Una barra con una sola salida es la de union de un `fork` que
            // ya se escribio, o una suelta: no abre nada.
            return edges.first().map(|e| e.target.node.as_str());
        }

        let exits: Vec<&'a str> = edges.iter().map(|e| e.target.node.as_str()).collect();
        let union = self.reunion(&exits);

        let indent = self.indent(depth);
        let _ = writeln!(out, "{indent}fork");

        for (index, edge) in edges.iter().enumerate() {
            if index > 0 {
                let _ = writeln!(out, "{indent}fork again");
            }
            self.mark(edge);
            self.region(Some(edge.target.node.as_str()), union, depth + 1, out);
        }

        // Si los caminos se juntan en otra barra, es una union; si se juntan
        // en un paso cualquiera, se juntaron sin barra.
        let joins = union.and_then(|id| self.node_of(id)).map(|n| n.shape) == Some(NodeShape::Bar);

        if joins {
            let _ = writeln!(out, "{indent}end fork");
            let union = union?;
            self.done.insert(union);

            let exit = self.forward(union);
            let exit = exit.first()?;
            self.mark(exit);
            return Some(exit.target.node.as_str());
        }

        let _ = writeln!(out, "{indent}end merge");
        union
    }

    fn split(&mut self, id: &'a str, depth: usize, out: &mut String) -> Option<&'a str> {
        let edges = self.forward(id);
        let exits: Vec<&'a str> = edges.iter().map(|e| e.target.node.as_str()).collect();
        let union = self.reunion(&exits);

        let indent = self.indent(depth);
        let _ = writeln!(out, "{indent}split");

        for (index, edge) in edges.iter().enumerate() {
            if index > 0 {
                let _ = writeln!(out, "{indent}split again");
            }
            self.mark(edge);
            self.region(Some(edge.target.node.as_str()), union, depth + 1, out);
        }

        let _ = writeln!(out, "{indent}end split");
        union
    }

    /* --- Particiones ------------------------------------------------------ */

    /// Cadena de particiones de un nodo, de fuera a dentro.
    fn chain(&self, id: Option<&'a str>) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut cursor = id;

        while let Some(current) = cursor {
            out.push(current);
            cursor = self.node_of(current).and_then(|n| n.parent.as_deref());
            if out.len() > self.doc.nodes.len() {
                break;
            }
        }

        out.reverse();
        out
    }

    fn enter_partition(&mut self, parent: Option<&'a str>, depth: usize, out: &mut String) {
        let wanted = self.chain(parent);

        let common = self
            .open
            .iter()
            .zip(wanted.iter())
            .take_while(|(a, b)| a == b)
            .count();

        self.close_partitions(common, depth, out);

        for id in &wanted[common..] {
            let name = self.caption(id);
            let indent = self.indent(depth);
            let _ = writeln!(out, "{indent}partition \"{name}\" {{");
            self.open.push(id);
        }
    }

    fn close_partitions(&mut self, down_to: usize, depth: usize, out: &mut String) {
        while self.open.len() > down_to {
            self.open.pop();
            let _ = writeln!(out, "{}}}", self.indent(depth));
        }
    }

    fn caption(&self, id: &str) -> String {
        self.node_of(id)
            .map(|n| escape(&n.label))
            .unwrap_or_default()
    }
}

/* --- Vueltas atras ---------------------------------------------------------- */

/// Marca las aristas que cierran un ciclo.
///
/// Es un recorrido en profundidad con los tres colores de siempre: una arista
/// que apunta a un nodo todavia abierto en el camino es una vuelta atras. La
/// pila es explicita para que un diagrama grande no agote la del programa.
fn back_edges<'a>(
    doc: &'a Document,
    out_edges: &HashMap<&'a str, Vec<&'a Edge>>,
) -> HashSet<&'a str> {
    enum Step<'a> {
        Enter(&'a str, Option<&'a str>),
        Leave(&'a str),
    }

    const OPEN: u8 = 1;
    const CLOSED: u8 = 2;

    let mut back = HashSet::new();
    let mut colour: HashMap<&'a str, u8> = HashMap::new();

    // Se empieza por los arranques para que las vueltas atras salgan donde el
    // flujo las tiene, y no donde caiga el primer nodo de la lista.
    let starts = doc
        .nodes
        .iter()
        .filter(|n| n.shape == NodeShape::InitialNode)
        .chain(doc.nodes.iter());

    for start in starts {
        if colour.contains_key(start.id.as_str()) {
            continue;
        }

        let mut stack = vec![Step::Enter(start.id.as_str(), None)];

        while let Some(step) = stack.pop() {
            match step {
                Step::Enter(id, via) => {
                    match colour.get(id) {
                        Some(&OPEN) => {
                            if let Some(edge) = via {
                                back.insert(edge);
                            }
                            continue;
                        }
                        Some(_) => continue,
                        None => {}
                    }

                    colour.insert(id, OPEN);
                    stack.push(Step::Leave(id));

                    for edge in out_edges.get(id).into_iter().flatten().rev() {
                        stack.push(Step::Enter(edge.target.node.as_str(), Some(&edge.id)));
                    }
                }
                Step::Leave(id) => {
                    colour.insert(id, CLOSED);
                }
            }
        }
    }

    back
}

/* --- Texto ------------------------------------------------------------------ */

/// Como se escribe un paso del flujo.
fn step_text(node: &Node, indent: &str) -> String {
    match node.shape {
        NodeShape::InitialNode => "start".to_string(),
        NodeShape::FinalNode => "stop".to_string(),
        // Una accion de varias lineas se escribe en varias lineas: el
        // importador las vuelve a juntar.
        _ => format!(
            ":{};",
            escape(&node.label).replace('\n', &format!("\n{indent}"))
        ),
    }
}

fn condition(node: &Node) -> String {
    escape(&node.label).replace('\n', " ")
}

/// `is (si)`, ` (no)` o nada, segun haya etiqueta y palabra que la una.
fn tail_label(word: &str, label: Option<&str>) -> String {
    match label.map(str::trim).filter(|l| !l.is_empty()) {
        Some(label) if word.is_empty() => format!(" ({label})"),
        Some(label) => format!(" {word} ({label})"),
        None => String::new(),
    }
}

/// Los parentesis de una condicion y las comillas de un nombre se cuentan al
/// leer, asi que los sueltos se quitan.
fn escape(text: &str) -> String {
    text.replace('"', "'")
}

#[cfg(test)]
mod tests;
