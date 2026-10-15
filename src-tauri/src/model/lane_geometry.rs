//! Geometria de un diagrama de secuencia.
//!
//! Convierte el guion —un orden— en coordenadas. Es la **unica** definicion
//! de como se reparte un diagrama de secuencia: la consumen tanto el lienzo
//! de carriles del frontend como el exportador de SVG, igual que
//! `shape_geometry` hace con los contornos.
//! `tests/golden/lane-layout.json` vigila que la copia de TypeScript siga
//! coincidiendo.
//!
//! Aqui no se mide texto. Los anchos y altos entran ya medidos por el
//! navegador (`Measures`) y lo que se hace con ellos es aritmetica, que es
//! justo lo que se puede tener dos veces sin que diverja.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::geometry::{Point, Rect, Size};
use super::sequence::{BlockKind, MessageKind, SequenceScript, Step};

/* --- Constantes ---------------------------------------------------------- */

/// Margen alrededor de todo el diagrama.
pub const MARGIN: f64 = 24.0;
/// Alto de la caja de cabecera de un participante.
pub const HEADER_HEIGHT: f64 = 40.0;
/// Ancho minimo de una cabecera, para que las cortas no queden ridiculas.
pub const MIN_HEADER_WIDTH: f64 = 90.0;
/// Hueco libre minimo entre dos cabeceras contiguas.
pub const MIN_GAP: f64 = 40.0;
/// Aire entre el texto de un mensaje y las lineas de vida que une.
pub const TEXT_PADDING: f64 = 12.0;

/// Separacion vertical entre un paso y el siguiente.
pub const STEP_GAP: f64 = 38.0;
/// Hueco entre la cabecera y el primer paso.
pub const FIRST_STEP_GAP: f64 = 30.0;
/// Cuanto sobresale la linea de vida por debajo del ultimo paso.
pub const TAIL: f64 = 28.0;

/// Ancho del lazo de un mensaje a uno mismo.
pub const SELF_LOOP_WIDTH: f64 = 44.0;
/// Alto del lazo de un mensaje a uno mismo.
pub const SELF_LOOP_HEIGHT: f64 = 26.0;

/// Ancho de la barra de ejecucion.
pub const ACTIVATION_WIDTH: f64 = 10.0;
/// Desplazamiento de cada barra anidada respecto de la de fuera.
pub const ACTIVATION_OFFSET: f64 = 5.0;
/// Lo que sobresale una barra por debajo del mensaje que la cierra.
pub const ACTIVATION_TAIL: f64 = 8.0;

/// Aire entre el borde de un marco y los lados del diagrama.
pub const FRAME_MARGIN: f64 = 10.0;
/// Cuanto se mete hacia dentro cada marco anidado.
pub const FRAME_INSET: f64 = 10.0;
/// Hueco entre el borde superior de un marco y su primer paso.
pub const FRAME_TOP: f64 = 26.0;
/// Hueco entre el ultimo paso de un marco y su borde inferior.
pub const FRAME_BOTTOM: f64 = 14.0;
/// Hueco que ocupa el separador entre dos ramas.
pub const DIVIDER_GAP: f64 = 24.0;

/// Relleno de la caja de una nota.
pub const NOTE_PADDING: f64 = 8.0;
/// Separacion entre una nota y la linea de vida a la que se pega.
pub const NOTE_GAP: f64 = 12.0;

/* --- Entrada ------------------------------------------------------------- */

/// Lo que el navegador ha medido y la geometria necesita.
///
/// Las claves son ids: de nodo para las cabeceras, de paso para los textos.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Measures {
    #[serde(default)]
    pub headers: BTreeMap<String, Size>,
    #[serde(default)]
    pub texts: BTreeMap<String, Size>,
}

impl Measures {
    fn header(&self, node: &str) -> Size {
        self.headers
            .get(node)
            .copied()
            .unwrap_or(Size::new(MIN_HEADER_WIDTH, HEADER_HEIGHT))
    }

    fn text(&self, id: &str) -> Size {
        self.texts.get(id).copied().unwrap_or(Size::new(0.0, 0.0))
    }
}

/* --- Salida -------------------------------------------------------------- */

/// Un participante ya colocado.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Lane {
    pub node: String,
    /// Centro de la linea de vida.
    pub x: f64,
    pub header: Rect,
    /// Donde empieza y acaba la linea de vida.
    pub top: f64,
    pub bottom: f64,
    /// Lo destruyo un mensaje: la linea acaba en una cruz.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub destroyed: bool,
}

/// Una flecha ya colocada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Arrow {
    pub id: String,
    /// Nodos que une. El lienzo los usa para resaltar, y el reparto para
    /// apoyar la flecha en el borde de la barra de ejecucion.
    pub from: String,
    pub to: String,
    pub kind: MessageKind,
    pub from_x: f64,
    pub to_x: f64,
    pub y: f64,
    /// Alto del lazo, si el mensaje va de un participante a si mismo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub loop_height: Option<f64>,
    /// Donde empieza el texto, alineado a la izquierda sobre la linea.
    pub text_at: Point,
}

/// Una barra de ejecucion.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Bar {
    pub node: String,
    pub rect: Rect,
}

/// El separador entre dos ramas de un marco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Divider {
    /// Id de la rama que abre. Con el se busca su texto medido.
    pub id: String,
    pub y: f64,
    pub label: String,
}

/// Un marco `alt`, `loop`, `par`...
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Frame {
    pub id: String,
    pub kind: BlockKind,
    pub rect: Rect,
    /// Condicion de la primera rama.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    #[serde(default)]
    pub dividers: Vec<Divider>,
}

/// Una nota ya colocada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct NoteBox {
    pub id: String,
    pub rect: Rect,
}

/// El diagrama entero, repartido.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct LaneLayout {
    #[serde(default)]
    pub lanes: Vec<Lane>,
    #[serde(default)]
    pub arrows: Vec<Arrow>,
    #[serde(default)]
    pub bars: Vec<Bar>,
    #[serde(default)]
    pub frames: Vec<Frame>,
    #[serde(default)]
    pub notes: Vec<NoteBox>,
    pub size: Size,
}

impl LaneLayout {
    pub fn lane(&self, node: &str) -> Option<&Lane> {
        self.lanes.iter().find(|lane| lane.node == node)
    }
}

/* --- Reparto ------------------------------------------------------------- */

/// Reparte el diagrama.
///
/// `participants` son los ids de los nodos en el orden en que van de
/// izquierda a derecha.
pub fn layout(participants: &[String], script: &SequenceScript, measures: &Measures) -> LaneLayout {
    if participants.is_empty() {
        return LaneLayout {
            size: Size::new(MARGIN * 2.0, MARGIN * 2.0),
            ..LaneLayout::default()
        };
    }

    let index: BTreeMap<&str, usize> = participants
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();

    let xs = horizontal(participants, script, measures, &index);
    let mut pass = Vertical::new(participants, measures, &index, &xs);
    pass.walk(&script.steps, 0);

    pass.finish(participants, measures, &xs)
}

/* --- Reparto horizontal -------------------------------------------------- */

/// Decide la `x` de cada linea de vida.
///
/// Parte de los huecos minimos que imponen las cabeceras y los va ensanchando
/// hasta que quepa el texto de cada mensaje. El reparto del deficit es a
/// partes iguales entre los huecos que el mensaje cruza, y se hace en orden
/// de lectura: es determinista, que es lo que necesita el fichero dorado, y
/// basta para que nada se solape.
fn horizontal(
    participants: &[String],
    script: &SequenceScript,
    measures: &Measures,
    index: &BTreeMap<&str, usize>,
) -> Vec<f64> {
    let count = participants.len();

    let halves: Vec<f64> = participants
        .iter()
        .map(|id| measures.header(id).width.max(MIN_HEADER_WIDTH) / 2.0)
        .collect();

    // Hueco entre el centro de una linea de vida y el de la siguiente.
    let mut gaps: Vec<f64> = (0..count.saturating_sub(1))
        .map(|i| halves[i] + halves[i + 1] + MIN_GAP)
        .collect();

    for message in script.messages() {
        let (Some(&from), Some(&to)) = (
            index.get(message.from.as_str()),
            index.get(message.to.as_str()),
        ) else {
            continue;
        };

        let text = measures.text(&message.id).width;

        // Un mensaje a uno mismo no cruza ningun hueco: necesita sitio a su
        // derecha, que sale del hueco siguiente o del margen si es el ultimo.
        if from == to {
            let needed = SELF_LOOP_WIDTH + text + TEXT_PADDING;
            if from < gaps.len() {
                gaps[from] = gaps[from].max(halves[from] + needed);
            }
            continue;
        }

        let (left, right) = (from.min(to), from.max(to));
        let needed = text + TEXT_PADDING * 2.0;
        let current: f64 = gaps[left..right].iter().sum();

        if needed > current {
            let share = (needed - current) / (right - left) as f64;
            for gap in &mut gaps[left..right] {
                *gap += share;
            }
        }
    }

    let mut xs = Vec::with_capacity(count);
    let mut x = MARGIN + halves[0];
    xs.push(x);
    for gap in &gaps {
        x += gap;
        xs.push(x);
    }

    xs
}

/* --- Reparto vertical ---------------------------------------------------- */

/// Barra mas interna de un participante que este abierta a esa altura.
///
/// La mas interna es la de mayor `x`: cada anidamiento se desplaza a la
/// derecha, y es a esa a la que llega la flecha.
fn innermost(bars: &[Bar], node: &str, y: f64) -> Option<Rect> {
    bars.iter()
        .filter(|bar| bar.node == node && bar.rect.y <= y && y <= bar.rect.bottom())
        .map(|bar| bar.rect)
        .reduce(|a, b| if b.x > a.x { b } else { a })
}

/// Una barra de ejecucion todavia abierta.
struct Open {
    node: String,
    top: f64,
    depth: usize,
}

struct Vertical<'a> {
    measures: &'a Measures,
    index: &'a BTreeMap<&'a str, usize>,
    xs: &'a [f64],

    /// Cursor vertical.
    y: f64,
    arrows: Vec<Arrow>,
    bars: Vec<Bar>,
    frames: Vec<Frame>,
    notes: Vec<NoteBox>,

    /// Pila de barras abiertas por participante.
    stacks: BTreeMap<String, Vec<Open>>,
    /// Participantes que nacen con un mensaje `create`, y donde.
    born: BTreeMap<String, f64>,
    /// Participantes destruidos, y donde.
    died: BTreeMap<String, f64>,
    /// Hasta donde llega el marco mas ancho; fija el ancho del diagrama.
    widest_frame: f64,
    /// Borde derecho del contenido: el canto de la ultima cabecera. Los
    /// marcos llegan hasta ahi, no hasta la ultima linea de vida, o la punta
    /// de una flecha aterrizaria justo encima del borde del marco.
    right_edge: f64,
}

impl<'a> Vertical<'a> {
    fn new(
        participants: &[String],
        measures: &'a Measures,
        index: &'a BTreeMap<&'a str, usize>,
        xs: &'a [f64],
    ) -> Self {
        let header_height = participants
            .iter()
            .map(|id| measures.header(id).height)
            .fold(HEADER_HEIGHT, f64::max);

        let right_edge = participants
            .last()
            .zip(xs.last())
            .map(|(id, &x)| x + measures.header(id).width.max(MIN_HEADER_WIDTH) / 2.0)
            .unwrap_or(0.0);

        Vertical {
            measures,
            index,
            xs,
            y: MARGIN + header_height + FIRST_STEP_GAP,
            arrows: Vec::new(),
            bars: Vec::new(),
            frames: Vec::new(),
            notes: Vec::new(),
            stacks: BTreeMap::new(),
            born: BTreeMap::new(),
            died: BTreeMap::new(),
            widest_frame: 0.0,
            right_edge,
        }
    }

    fn x_of(&self, node: &str) -> Option<f64> {
        self.index.get(node).and_then(|&i| self.xs.get(i)).copied()
    }

    fn walk(&mut self, steps: &[Step], depth: usize) {
        for step in steps {
            match step {
                Step::Message(message) => self.message(message),
                Step::Note(note) => self.note(note),
                Step::Block(block) => self.block(block, depth),
            }
        }
    }

    fn message(&mut self, message: &super::sequence::Message) {
        let (Some(from_x), Some(to_x)) = (self.x_of(&message.from), self.x_of(&message.to)) else {
            return;
        };

        self.y += STEP_GAP;
        let y = self.y;

        // Cerrar antes de abrir: `a -> b` que desactiva `a` y activa `b` son
        // dos cosas distintas y las dos ocurren en esta misma linea.
        if message.deactivate {
            self.close(&message.from, y);
        }

        let text = self.measures.text(&message.id);
        let loop_height = message.is_self().then_some(SELF_LOOP_HEIGHT);

        let text_at = if message.is_self() {
            Point::new(from_x + SELF_LOOP_WIDTH + 6.0, y - text.height / 2.0)
        } else {
            // Centrado sobre el tramo, justo encima de la linea.
            Point::new(
                (from_x + to_x) / 2.0 - text.width / 2.0,
                y - text.height - 6.0,
            )
        };

        self.arrows.push(Arrow {
            id: message.id.clone(),
            from: message.from.clone(),
            to: message.to.clone(),
            kind: message.kind,
            from_x,
            to_x,
            y,
            loop_height,
            text_at,
        });

        match message.kind {
            MessageKind::Create => {
                self.born.insert(message.to.clone(), y);
            }
            MessageKind::Destroy => {
                self.died.insert(message.to.clone(), y);
                // Lo que estuviera corriendo en el destruido acaba aqui.
                while self.stacks.get(&message.to).is_some_and(|s| !s.is_empty()) {
                    self.close(&message.to, y);
                }
            }
            _ => {}
        }

        if message.activate {
            let depth = self.stacks.get(&message.to).map_or(0, Vec::len);
            self.stacks
                .entry(message.to.clone())
                .or_default()
                .push(Open {
                    node: message.to.clone(),
                    top: y,
                    depth,
                });
        }

        if message.is_self() {
            self.y += SELF_LOOP_HEIGHT;
        }
    }

    /// Cierra la barra mas interna de un participante.
    fn close(&mut self, node: &str, y: f64) {
        let Some(open) = self.stacks.get_mut(node).and_then(Vec::pop) else {
            return;
        };
        self.push_bar(open, y + ACTIVATION_TAIL);
    }

    fn push_bar(&mut self, open: Open, bottom: f64) {
        let Some(x) = self.x_of(&open.node) else {
            return;
        };

        self.bars.push(Bar {
            node: open.node,
            rect: Rect::new(
                x - ACTIVATION_WIDTH / 2.0 + open.depth as f64 * ACTIVATION_OFFSET,
                open.top,
                ACTIVATION_WIDTH,
                (bottom - open.top).max(ACTIVATION_WIDTH),
            ),
        });
    }

    fn note(&mut self, note: &super::sequence::SequenceNote) {
        let text = self.measures.text(&note.id);
        let width = text.width + NOTE_PADDING * 2.0;
        let height = text.height + NOTE_PADDING * 2.0;

        let anchors: Vec<f64> = note
            .participants
            .iter()
            .filter_map(|id| self.x_of(id))
            .collect();
        let Some(&first) = anchors.first() else {
            return;
        };
        let last = *anchors.last().unwrap_or(&first);

        let x = match note.placement {
            super::sequence::NotePlacement::LeftOf => first - NOTE_GAP - width,
            super::sequence::NotePlacement::RightOf => last + NOTE_GAP,
            // Sobre una linea va centrada; sobre varias, cubre de la primera
            // a la ultima.
            super::sequence::NotePlacement::Over if anchors.len() > 1 => {
                let span = (last - first + width).max(width);
                self.widest_frame = self.widest_frame.max(first - NOTE_PADDING + span);
                (first + last) / 2.0 - span / 2.0
            }
            super::sequence::NotePlacement::Over => first - width / 2.0,
        };

        let width = match note.placement {
            super::sequence::NotePlacement::Over if anchors.len() > 1 => {
                (last - first + width).max(width)
            }
            _ => width,
        };

        self.y += NOTE_GAP;
        self.notes.push(NoteBox {
            id: note.id.clone(),
            rect: Rect::new(x, self.y, width, height),
        });
        self.widest_frame = self.widest_frame.max(x + width);
        self.y += height;
    }

    fn block(&mut self, block: &super::sequence::Block, depth: usize) {
        let top = self.y + STEP_GAP / 2.0;
        self.y = top + FRAME_TOP;

        let mut dividers = Vec::new();

        for (position, branch) in block.branches.iter().enumerate() {
            if position > 0 {
                self.y += DIVIDER_GAP;
                dividers.push(Divider {
                    id: branch.id.clone(),
                    y: self.y,
                    label: branch.label.clone(),
                });
            }
            self.walk(&branch.steps, depth + 1);
        }

        self.y += FRAME_BOTTOM;

        let inset = FRAME_MARGIN + FRAME_INSET * depth as f64;
        let left = MARGIN + inset;
        let right = self.right_edge - inset + FRAME_MARGIN;

        self.widest_frame = self.widest_frame.max(right);

        self.frames.push(Frame {
            id: block.id.clone(),
            kind: block.kind,
            rect: Rect::new(left, top, (right - left).max(0.0), self.y - top),
            label: block
                .branches
                .first()
                .map(|b| b.label.clone())
                .unwrap_or_default(),
            dividers,
        });
    }

    /// Apoya cada flecha en el borde de la barra de ejecucion que toque.
    ///
    /// Sin esto una flecha entra hasta el centro de la linea de vida y
    /// atraviesa la barra, que es justo lo que la barra quiere mostrar que no
    /// pasa: la llamada llega al borde de la ejecucion, no a su interior.
    fn trim_arrows(&mut self) {
        let bars = std::mem::take(&mut self.bars);

        for arrow in &mut self.arrows {
            let origin = innermost(&bars, &arrow.from, arrow.y);
            let destination = innermost(&bars, &arrow.to, arrow.y);

            if arrow.from == arrow.to {
                // El lazo sale y vuelve por el mismo lado.
                if let Some(rect) = origin {
                    arrow.from_x = rect.right();
                    arrow.to_x = rect.right();
                }
                continue;
            }

            let rightward = arrow.from_x < arrow.to_x;
            if let Some(rect) = origin {
                arrow.from_x = if rightward { rect.right() } else { rect.x };
            }
            if let Some(rect) = destination {
                arrow.to_x = if rightward { rect.x } else { rect.right() };
            }
        }

        self.bars = bars;
    }

    fn finish(mut self, participants: &[String], measures: &Measures, xs: &[f64]) -> LaneLayout {
        let bottom = self.y + TAIL;

        // Lo que quedo corriendo al acabar el guion se cierra al final.
        let pending: Vec<Open> = std::mem::take(&mut self.stacks)
            .into_values()
            .flatten()
            .collect();
        for open in pending {
            self.push_bar(open, bottom);
        }
        // El orden de las barras tiene que ser estable: salen de un mapa.
        self.bars.sort_by(|a, b| {
            (a.rect.y, a.rect.x)
                .partial_cmp(&(b.rect.y, b.rect.x))
                .unwrap()
        });

        self.trim_arrows();

        let header_height = participants
            .iter()
            .map(|id| measures.header(id).height)
            .fold(HEADER_HEIGHT, f64::max);

        let lanes: Vec<Lane> = participants
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let width = measures.header(node).width.max(MIN_HEADER_WIDTH);
                let x = xs[i];

                // Un participante creado a mitad del diagrama no tiene
                // cabecera arriba: nace donde le llega el mensaje.
                let header_y = match self.born.get(node) {
                    Some(&y) => y - header_height / 2.0,
                    None => MARGIN,
                };

                Lane {
                    node: node.clone(),
                    x,
                    header: Rect::new(x - width / 2.0, header_y, width, header_height),
                    top: header_y + header_height,
                    bottom: self.died.get(node).copied().unwrap_or(bottom),
                    destroyed: self.died.contains_key(node),
                }
            })
            .collect();

        let right = lanes
            .iter()
            .map(|lane| lane.header.right())
            .fold(self.widest_frame, f64::max);

        LaneLayout {
            lanes,
            arrows: self.arrows,
            bars: self.bars,
            frames: self.frames,
            notes: self.notes,
            size: Size::new(right + MARGIN, bottom + MARGIN),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sequence::{Block, Branch, Message, NotePlacement, SequenceNote};

    fn ids(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    /// Medidas uniformes: cabeceras de 100 y los textos que se indiquen.
    fn measures(participants: &[&str], texts: &[(&str, f64)]) -> Measures {
        let mut m = Measures::default();
        for id in participants {
            m.headers.insert(id.to_string(), Size::new(100.0, 40.0));
        }
        for (id, width) in texts {
            m.texts.insert(id.to_string(), Size::new(*width, 14.0));
        }
        m
    }

    fn message(id: &str, from: &str, to: &str) -> Step {
        Step::Message(Message::new(id, from, to, MessageKind::Sync))
    }

    fn script(steps: Vec<Step>) -> SequenceScript {
        SequenceScript { steps }
    }

    fn run(participants: &[&str], steps: Vec<Step>, texts: &[(&str, f64)]) -> LaneLayout {
        layout(
            &ids(participants),
            &script(steps),
            &measures(participants, texts),
        )
    }

    /* --- Carriles -------------------------------------------------------- */

    #[test]
    fn las_lineas_de_vida_van_de_izquierda_a_derecha_en_orden() {
        let out = run(&["a", "b", "c"], vec![], &[]);

        assert_eq!(out.lanes.len(), 3);
        assert!(out.lanes[0].x < out.lanes[1].x);
        assert!(out.lanes[1].x < out.lanes[2].x);
    }

    #[test]
    fn una_cabecera_ancha_separa_mas_su_carril() {
        let mut wide = measures(&["a", "b"], &[]);
        wide.headers.insert("a".into(), Size::new(300.0, 40.0));

        let estrecho = layout(
            &ids(&["a", "b"]),
            &script(vec![]),
            &measures(&["a", "b"], &[]),
        );
        let ancho = layout(&ids(&["a", "b"]), &script(vec![]), &wide);

        let separacion = |l: &LaneLayout| l.lanes[1].x - l.lanes[0].x;
        assert!(separacion(&ancho) > separacion(&estrecho));
    }

    #[test]
    fn la_cabecera_esta_centrada_en_su_linea_de_vida() {
        let out = run(&["a"], vec![], &[]);
        let lane = &out.lanes[0];

        assert_eq!(lane.header.x + lane.header.width / 2.0, lane.x);
    }

    #[test]
    fn un_diagrama_sin_participantes_no_revienta() {
        let out = layout(&[], &script(vec![]), &Measures::default());

        assert!(out.lanes.is_empty());
        assert_eq!(out.size.width, MARGIN * 2.0);
    }

    /* --- Mensajes -------------------------------------------------------- */

    #[test]
    fn los_mensajes_bajan_en_el_orden_del_guion() {
        let out = run(
            &["a", "b"],
            vec![message("m1", "a", "b"), message("m2", "b", "a")],
            &[],
        );

        assert_eq!(out.arrows.len(), 2);
        assert!(out.arrows[0].y < out.arrows[1].y);
    }

    #[test]
    fn el_primer_mensaje_queda_por_debajo_de_las_cabeceras() {
        let out = run(&["a", "b"], vec![message("m", "a", "b")], &[]);

        assert!(out.arrows[0].y > out.lanes[0].top);
    }

    #[test]
    fn un_texto_largo_ensancha_el_hueco_que_cruza() {
        let corto = run(&["a", "b"], vec![message("m", "a", "b")], &[("m", 20.0)]);
        let largo = run(&["a", "b"], vec![message("m", "a", "b")], &[("m", 400.0)]);

        let ancho = |l: &LaneLayout| l.lanes[1].x - l.lanes[0].x;
        assert!(ancho(&largo) > ancho(&corto));
        // Y queda sitio para el texto a los dos lados.
        assert!(ancho(&largo) >= 400.0 + TEXT_PADDING * 2.0);
    }

    #[test]
    fn el_deficit_se_reparte_entre_los_huecos_que_cruza() {
        // Un mensaje de `a` a `c` ensancha los dos huecos, no solo uno.
        let out = run(
            &["a", "b", "c"],
            vec![message("m", "a", "c")],
            &[("m", 600.0)],
        );

        let primero = out.lanes[1].x - out.lanes[0].x;
        let segundo = out.lanes[2].x - out.lanes[1].x;

        assert!((primero - segundo).abs() < 0.001, "{primero} vs {segundo}");
    }

    #[test]
    fn un_mensaje_a_uno_mismo_lleva_lazo_y_ocupa_mas_alto() {
        let solo = run(&["a", "b"], vec![message("m", "a", "a")], &[]);
        let cruzado = run(&["a", "b"], vec![message("m", "a", "b")], &[]);

        assert_eq!(solo.arrows[0].loop_height, Some(SELF_LOOP_HEIGHT));
        assert_eq!(cruzado.arrows[0].loop_height, None);
        assert!(solo.size.height > cruzado.size.height);
    }

    #[test]
    fn un_mensaje_a_uno_mismo_reserva_sitio_a_su_derecha() {
        let out = run(&["a", "b"], vec![message("m", "a", "a")], &[("m", 200.0)]);

        let hueco = out.lanes[1].x - out.lanes[0].x;
        assert!(hueco >= SELF_LOOP_WIDTH + 200.0);
    }

    #[test]
    fn un_mensaje_a_un_participante_que_no_existe_se_ignora() {
        let out = run(&["a"], vec![message("m", "a", "fantasma")], &[]);

        assert!(out.arrows.is_empty());
    }

    /* --- Barras de ejecucion --------------------------------------------- */

    #[test]
    fn activar_y_desactivar_dibuja_una_barra_entre_los_dos_mensajes() {
        let llamada = Step::Message(Message::new("m1", "a", "b", MessageKind::Sync).activating());
        let vuelta = Step::Message(Message::new("m2", "b", "a", MessageKind::Reply).deactivating());

        let out = run(&["a", "b"], vec![llamada, vuelta], &[]);

        assert_eq!(out.bars.len(), 1);
        let bar = &out.bars[0];
        assert_eq!(bar.node, "b");
        assert_eq!(bar.rect.y, out.arrows[0].y);
        assert!(bar.rect.bottom() > out.arrows[1].y);
    }

    #[test]
    fn las_barras_anidadas_se_desplazan_a_la_derecha() {
        let steps = vec![
            Step::Message(Message::new("m1", "a", "b", MessageKind::Sync).activating()),
            Step::Message(Message::new("m2", "a", "b", MessageKind::Sync).activating()),
        ];

        let out = run(&["a", "b"], steps, &[]);

        assert_eq!(out.bars.len(), 2);
        assert!(out.bars[1].rect.x > out.bars[0].rect.x);
    }

    #[test]
    fn una_barra_sin_cerrar_llega_hasta_el_final() {
        let steps = vec![Step::Message(
            Message::new("m", "a", "b", MessageKind::Sync).activating(),
        )];

        let out = run(&["a", "b"], steps, &[]);

        assert_eq!(out.bars.len(), 1);
        assert!(out.bars[0].rect.bottom() >= out.lanes[1].bottom);
    }

    #[test]
    fn desactivar_sin_haber_activado_no_dibuja_nada() {
        let steps = vec![Step::Message(
            Message::new("m", "a", "b", MessageKind::Sync).deactivating(),
        )];

        assert!(run(&["a", "b"], steps, &[]).bars.is_empty());
    }

    #[test]
    fn una_flecha_se_apoya_en_el_borde_de_la_barra_y_no_la_atraviesa() {
        let steps = vec![
            Step::Message(Message::new("m1", "a", "b", MessageKind::Sync).activating()),
            Step::Message(Message::new("m2", "a", "b", MessageKind::Sync)),
        ];

        let out = run(&["a", "b"], steps, &[]);
        let bar = &out.bars[0];

        // La segunda flecha llega cuando la barra ya esta abierta.
        assert_eq!(out.arrows[1].to_x, bar.rect.x);
        assert!(out.arrows[1].to_x < out.lanes[1].x);
    }

    #[test]
    fn una_flecha_hacia_la_izquierda_se_apoya_en_el_borde_contrario() {
        let steps = vec![
            Step::Message(Message::new("m1", "b", "a", MessageKind::Sync).activating()),
            Step::Message(Message::new("m2", "b", "a", MessageKind::Sync)),
        ];

        let out = run(&["a", "b"], steps, &[]);
        let bar = &out.bars[0];

        assert_eq!(out.arrows[1].to_x, bar.rect.right());
        assert!(out.arrows[1].to_x > out.lanes[0].x);
    }

    #[test]
    fn sin_barra_la_flecha_llega_al_centro_de_la_linea_de_vida() {
        let out = run(&["a", "b"], vec![message("m", "a", "b")], &[]);

        assert_eq!(out.arrows[0].from_x, out.lanes[0].x);
        assert_eq!(out.arrows[0].to_x, out.lanes[1].x);
    }

    /* --- Crear y destruir ------------------------------------------------ */

    #[test]
    fn un_participante_creado_a_mitad_tiene_la_cabecera_a_esa_altura() {
        let steps = vec![
            message("m1", "a", "a"),
            Step::Message(Message::new("m2", "a", "b", MessageKind::Create)),
        ];

        let out = run(&["a", "b"], steps, &[]);
        let creado = out.lane("b").unwrap();

        assert!(creado.header.y > out.lane("a").unwrap().header.y);
        assert!(creado.top > out.arrows[0].y);
    }

    #[test]
    fn un_participante_destruido_acaba_su_linea_de_vida_ahi() {
        let steps = vec![
            Step::Message(Message::new("m1", "a", "b", MessageKind::Destroy)),
            message("m2", "a", "a"),
        ];

        let out = run(&["a", "b"], steps, &[]);
        let muerto = out.lane("b").unwrap();

        assert!(muerto.destroyed);
        assert_eq!(muerto.bottom, out.arrows[0].y);
        assert!(muerto.bottom < out.lane("a").unwrap().bottom);
    }

    /* --- Marcos ---------------------------------------------------------- */

    #[test]
    fn un_marco_envuelve_a_sus_pasos() {
        let steps = vec![Step::Block(Block {
            id: "b1".into(),
            kind: BlockKind::Opt,
            branches: vec![
                Branch::new("r1", "si hay stock").with_steps(vec![message("m", "a", "b")]),
            ],
        })];

        let out = run(&["a", "b"], steps, &[]);

        assert_eq!(out.frames.len(), 1);
        let frame = &out.frames[0];
        assert_eq!(frame.label, "si hay stock");
        assert!(frame.rect.y < out.arrows[0].y);
        assert!(frame.rect.bottom() > out.arrows[0].y);
    }

    #[test]
    fn un_alt_con_dos_ramas_lleva_un_separador() {
        let steps = vec![Step::Block(Block {
            id: "b1".into(),
            kind: BlockKind::Alt,
            branches: vec![
                Branch::new("r1", "ok").with_steps(vec![message("m1", "a", "b")]),
                Branch::new("r2", "error").with_steps(vec![message("m2", "a", "b")]),
            ],
        })];

        let out = run(&["a", "b"], steps, &[]);
        let frame = &out.frames[0];

        assert_eq!(frame.dividers.len(), 1);
        assert_eq!(frame.dividers[0].label, "error");
        // El separador cae entre los dos mensajes.
        assert!(frame.dividers[0].y > out.arrows[0].y);
        assert!(frame.dividers[0].y < out.arrows[1].y);
    }

    #[test]
    fn un_marco_anidado_se_mete_hacia_dentro() {
        let dentro = Step::Block(Block {
            id: "dentro".into(),
            kind: BlockKind::Opt,
            branches: vec![Branch::new("r2", "").with_steps(vec![message("m", "a", "b")])],
        });
        let steps = vec![Step::Block(Block {
            id: "fuera".into(),
            kind: BlockKind::Loop,
            branches: vec![Branch::new("r1", "3 veces").with_steps(vec![dentro])],
        })];

        let out = run(&["a", "b"], steps, &[]);

        let fuera = out.frames.iter().find(|f| f.id == "fuera").unwrap();
        let dentro = out.frames.iter().find(|f| f.id == "dentro").unwrap();

        assert!(dentro.rect.x > fuera.rect.x);
        assert!(dentro.rect.y > fuera.rect.y);
        assert!(dentro.rect.bottom() < fuera.rect.bottom());
    }

    /* --- Notas ----------------------------------------------------------- */

    #[test]
    fn una_nota_a_la_derecha_queda_pasada_la_linea_de_vida() {
        let steps = vec![Step::Note(SequenceNote {
            id: "n1".into(),
            placement: NotePlacement::RightOf,
            participants: ids(&["a"]),
            text: "ojo".into(),
        })];

        let out = run(&["a", "b"], steps, &[("n1", 60.0)]);

        assert_eq!(out.notes.len(), 1);
        assert!(out.notes[0].rect.x > out.lanes[0].x);
    }

    #[test]
    fn una_nota_sobre_dos_participantes_los_cubre_a_los_dos() {
        let steps = vec![Step::Note(SequenceNote {
            id: "n1".into(),
            placement: NotePlacement::Over,
            participants: ids(&["a", "b"]),
            text: "las dos".into(),
        })];

        let out = run(&["a", "b"], steps, &[("n1", 60.0)]);
        let rect = out.notes[0].rect;

        assert!(rect.x <= out.lanes[0].x);
        assert!(rect.right() >= out.lanes[1].x);
    }

    #[test]
    fn una_nota_empuja_hacia_abajo_lo_que_venga_detras() {
        let nota = Step::Note(SequenceNote {
            id: "n1".into(),
            placement: NotePlacement::Over,
            participants: ids(&["a"]),
            text: "ojo".into(),
        });

        let con_nota = run(
            &["a", "b"],
            vec![nota, message("m", "a", "b")],
            &[("n1", 60.0)],
        );
        let sin_nota = run(&["a", "b"], vec![message("m", "a", "b")], &[]);

        assert!(con_nota.arrows[0].y > sin_nota.arrows[0].y);
    }

    /* --- Tamano ---------------------------------------------------------- */

    #[test]
    fn el_diagrama_envuelve_todo_lo_que_contiene() {
        let out = run(
            &["a", "b"],
            vec![message("m1", "a", "b"), message("m2", "b", "a")],
            &[],
        );

        assert!(out.size.width > out.lanes[1].header.right());
        assert!(out.size.height > out.lanes[0].bottom);
    }

    #[test]
    fn el_reparto_es_estable_entre_llamadas() {
        // Las barras salen de un mapa; sin ordenarlas, el fichero dorado
        // cambiaria de una ejecucion a otra.
        let steps = || {
            vec![
                Step::Message(Message::new("m1", "a", "b", MessageKind::Sync).activating()),
                Step::Message(Message::new("m2", "b", "c", MessageKind::Sync).activating()),
                Step::Message(Message::new("m3", "c", "b", MessageKind::Reply).deactivating()),
            ]
        };

        let primero = run(&["a", "b", "c"], steps(), &[]);
        let segundo = run(&["a", "b", "c"], steps(), &[]);

        assert_eq!(primero, segundo);
    }
}
