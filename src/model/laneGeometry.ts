import type { Arrow } from "./generated/Arrow";
import type { Bar } from "./generated/Bar";
import type { Divider } from "./generated/Divider";
import type { Frame } from "./generated/Frame";
import type { Lane } from "./generated/Lane";
import type { LaneLayout } from "./generated/LaneLayout";
import type { Measures } from "./generated/Measures";
import type { Message } from "./generated/Message";
import type { NoteBox } from "./generated/NoteBox";
import type { Rect } from "./generated/Rect";
import type { SequenceNote } from "./generated/SequenceNote";
import type { SequenceScript } from "./generated/SequenceScript";
import type { Size } from "./generated/Size";
import type { Step } from "./generated/Step";

/**
 * Reparto de un diagrama de secuencia.
 *
 * Copia de `src-tauri/src/model/lane_geometry.rs`. Existe para que el lienzo
 * de carriles pueda repintar sin cruzar el puente IPC en cada cambio, igual
 * que `shapeGeometry.ts` hace con los contornos.
 *
 * Las dos implementaciones **tienen que dar el mismo resultado**:
 * `src-tauri/tests/golden/lane-layout.json` las enfrenta. Si tocas una, toca
 * la otra y regenera el fichero con
 * `UPDATE_GOLDEN=1 cargo test --manifest-path src-tauri/Cargo.toml golden`.
 *
 * Aqui no se mide texto: las medidas entran ya hechas en `Measures`. Lo que
 * se hace con ellas es aritmetica, que es justo lo que se puede tener dos
 * veces sin que diverja.
 */

/* --- Constantes. Espejo de las de Rust ----------------------------------- */

export const MARGIN = 24;
export const HEADER_HEIGHT = 40;
export const MIN_HEADER_WIDTH = 90;
export const MIN_GAP = 40;
export const TEXT_PADDING = 12;

export const STEP_GAP = 38;
export const FIRST_STEP_GAP = 30;
export const TAIL = 28;

export const SELF_LOOP_WIDTH = 44;
export const SELF_LOOP_HEIGHT = 26;

export const ACTIVATION_WIDTH = 10;
export const ACTIVATION_OFFSET = 5;
export const ACTIVATION_TAIL = 8;

export const FRAME_MARGIN = 10;
export const FRAME_INSET = 10;
export const FRAME_TOP = 26;
export const FRAME_BOTTOM = 14;
export const DIVIDER_GAP = 24;

export const NOTE_PADDING = 8;
export const NOTE_GAP = 12;

/* --- Utilidades ---------------------------------------------------------- */

const right = (r: Rect) => r.x + r.width;
const bottom = (r: Rect) => r.y + r.height;

function headerSize(measures: Measures, node: string): Size {
  return measures.headers[node] ?? { width: MIN_HEADER_WIDTH, height: HEADER_HEIGHT };
}

function textSize(measures: Measures, id: string): Size {
  return measures.texts[id] ?? { width: 0, height: 0 };
}

/** Recorre los pasos entrando en los marcos, en orden de lectura. */
export function walkSteps(steps: Step[]): Step[] {
  const out: Step[] = [];

  const visit = (list: Step[]) => {
    for (const step of list) {
      out.push(step);
      if (step.type === "block") {
        for (const branch of step.branches) visit(branch.steps);
      }
    }
  };

  visit(steps);
  return out;
}

/** Mensajes del guion, en orden. */
export function messagesOf(script: SequenceScript): Message[] {
  return walkSteps(script.steps).filter((s) => s.type === "message");
}

const isSelf = (m: { from: string; to: string }) => m.from === m.to;

/* --- Reparto ------------------------------------------------------------- */

export function layout(
  participants: string[],
  script: SequenceScript,
  measures: Measures,
): LaneLayout {
  if (participants.length === 0) {
    return {
      lanes: [],
      arrows: [],
      bars: [],
      frames: [],
      notes: [],
      size: { width: MARGIN * 2, height: MARGIN * 2 },
    };
  }

  const index = new Map(participants.map((id, i) => [id, i]));
  const xs = horizontal(participants, script, measures, index);

  return new Vertical(participants, measures, index, xs).run(script);
}

/**
 * Decide la `x` de cada linea de vida.
 *
 * Parte de los huecos minimos que imponen las cabeceras y los va ensanchando
 * hasta que quepa el texto de cada mensaje. El deficit se reparte a partes
 * iguales entre los huecos que el mensaje cruza, en orden de lectura.
 */
function horizontal(
  participants: string[],
  script: SequenceScript,
  measures: Measures,
  index: Map<string, number>,
): number[] {
  const halves = participants.map(
    (id) => Math.max(headerSize(measures, id).width, MIN_HEADER_WIDTH) / 2,
  );

  const gaps: number[] = [];
  for (let i = 0; i < participants.length - 1; i++) {
    gaps.push(halves[i]! + halves[i + 1]! + MIN_GAP);
  }

  for (const message of messagesOf(script)) {
    const from = index.get(message.from);
    const to = index.get(message.to);
    if (from === undefined || to === undefined) continue;

    const text = textSize(measures, message.id).width;

    // Un mensaje a uno mismo no cruza ningun hueco: necesita sitio a su
    // derecha, que sale del hueco siguiente o del margen si es el ultimo.
    if (from === to) {
      const needed = SELF_LOOP_WIDTH + text + TEXT_PADDING;
      if (from < gaps.length) {
        gaps[from] = Math.max(gaps[from]!, halves[from]! + needed);
      }
      continue;
    }

    const left = Math.min(from, to);
    const far = Math.max(from, to);
    const needed = text + TEXT_PADDING * 2;

    let current = 0;
    for (let i = left; i < far; i++) current += gaps[i]!;

    if (needed > current) {
      const share = (needed - current) / (far - left);
      for (let i = left; i < far; i++) gaps[i] = gaps[i]! + share;
    }
  }

  const xs = [MARGIN + halves[0]!];
  for (const gap of gaps) xs.push(xs[xs.length - 1]! + gap);

  return xs;
}

/** Una barra de ejecucion todavia abierta. */
interface Open {
  node: string;
  top: number;
  depth: number;
}

/**
 * Barra mas interna de un participante que este abierta a esa altura.
 *
 * La mas interna es la de mayor `x`: cada anidamiento se desplaza a la
 * derecha, y es a esa a la que llega la flecha.
 */
function innermost(bars: Bar[], node: string, y: number): Rect | undefined {
  let found: Rect | undefined;

  for (const bar of bars) {
    if (bar.node !== node || bar.rect.y > y || y > bottom(bar.rect)) continue;
    if (!found || bar.rect.x > found.x) found = bar.rect;
  }

  return found;
}

class Vertical {
  private y: number;
  private arrows: Arrow[] = [];
  private bars: Bar[] = [];
  private frames: Frame[] = [];
  private notes: NoteBox[] = [];

  /** Pila de barras abiertas por participante. */
  private stacks = new Map<string, Open[]>();
  private born = new Map<string, number>();
  private died = new Map<string, number>();
  private widestFrame = 0;

  private readonly headerHeight: number;
  private readonly rightEdge: number;

  constructor(
    private readonly participants: string[],
    private readonly measures: Measures,
    private readonly index: Map<string, number>,
    private readonly xs: number[],
  ) {
    this.headerHeight = participants.reduce(
      (acc, id) => Math.max(acc, headerSize(measures, id).height),
      HEADER_HEIGHT,
    );

    const last = participants[participants.length - 1];
    const lastX = xs[xs.length - 1];
    this.rightEdge =
      last !== undefined && lastX !== undefined
        ? lastX + Math.max(headerSize(measures, last).width, MIN_HEADER_WIDTH) / 2
        : 0;

    this.y = MARGIN + this.headerHeight + FIRST_STEP_GAP;
  }

  run(script: SequenceScript): LaneLayout {
    this.walk(script.steps, 0);
    return this.finish();
  }

  private xOf(node: string): number | undefined {
    const i = this.index.get(node);
    return i === undefined ? undefined : this.xs[i];
  }

  private walk(steps: Step[], depth: number) {
    for (const step of steps) {
      if (step.type === "message") this.message(step);
      else if (step.type === "note") this.note(step);
      else this.block(step, depth);
    }
  }

  private message(message: Step & { type: "message" }) {
    const fromX = this.xOf(message.from);
    const toX = this.xOf(message.to);
    if (fromX === undefined || toX === undefined) return;

    this.y += STEP_GAP;
    const y = this.y;

    // Cerrar antes de abrir: un mensaje que desactiva el origen y activa el
    // destino hace las dos cosas en esta misma linea.
    if (message.deactivate) this.close(message.from, y);

    const text = textSize(this.measures, message.id);
    const loop = isSelf(message);

    const textAt = loop
      ? { x: fromX + SELF_LOOP_WIDTH + 6, y: y - text.height / 2 }
      : { x: (fromX + toX) / 2 - text.width / 2, y: y - text.height - 6 };

    this.arrows.push({
      id: message.id,
      from: message.from,
      to: message.to,
      kind: message.kind,
      fromX,
      toX,
      y,
      ...(loop ? { loopHeight: SELF_LOOP_HEIGHT } : {}),
      textAt,
    });

    if (message.kind === "create") {
      this.born.set(message.to, y);
    } else if (message.kind === "destroy") {
      this.died.set(message.to, y);
      // Lo que estuviera corriendo en el destruido acaba aqui.
      while ((this.stacks.get(message.to)?.length ?? 0) > 0) this.close(message.to, y);
    }

    if (message.activate) {
      const stack = this.stacks.get(message.to) ?? [];
      stack.push({ node: message.to, top: y, depth: stack.length });
      this.stacks.set(message.to, stack);
    }

    if (loop) this.y += SELF_LOOP_HEIGHT;
  }

  /** Cierra la barra mas interna de un participante. */
  private close(node: string, y: number) {
    const open = this.stacks.get(node)?.pop();
    if (open) this.pushBar(open, y + ACTIVATION_TAIL);
  }

  private pushBar(open: Open, barBottom: number) {
    const x = this.xOf(open.node);
    if (x === undefined) return;

    this.bars.push({
      node: open.node,
      rect: {
        x: x - ACTIVATION_WIDTH / 2 + open.depth * ACTIVATION_OFFSET,
        y: open.top,
        width: ACTIVATION_WIDTH,
        height: Math.max(barBottom - open.top, ACTIVATION_WIDTH),
      },
    });
  }

  private note(note: SequenceNote) {
    const text = textSize(this.measures, note.id);
    const boxWidth = text.width + NOTE_PADDING * 2;
    const height = text.height + NOTE_PADDING * 2;

    const anchors = note.participants
      .map((id) => this.xOf(id))
      .filter((x): x is number => x !== undefined);

    const first = anchors[0];
    if (first === undefined) return;
    const last = anchors[anchors.length - 1] ?? first;

    const spans = note.placement === "over" && anchors.length > 1;
    const width = spans ? Math.max(last - first + boxWidth, boxWidth) : boxWidth;

    let x: number;
    if (note.placement === "leftOf") {
      x = first - NOTE_GAP - boxWidth;
    } else if (note.placement === "rightOf") {
      x = last + NOTE_GAP;
    } else if (spans) {
      this.widestFrame = Math.max(this.widestFrame, first - NOTE_PADDING + width);
      x = (first + last) / 2 - width / 2;
    } else {
      x = first - boxWidth / 2;
    }

    this.y += NOTE_GAP;
    this.notes.push({ id: note.id, rect: { x, y: this.y, width, height } });
    this.widestFrame = Math.max(this.widestFrame, x + width);
    this.y += height;
  }

  private block(block: Step & { type: "block" }, depth: number) {
    const top = this.y + STEP_GAP / 2;
    this.y = top + FRAME_TOP;

    const dividers: Divider[] = [];

    block.branches.forEach((branch, position) => {
      if (position > 0) {
        this.y += DIVIDER_GAP;
        dividers.push({ id: branch.id, y: this.y, label: branch.label ?? "" });
      }
      this.walk(branch.steps, depth + 1);
    });

    this.y += FRAME_BOTTOM;

    const inset = FRAME_MARGIN + FRAME_INSET * depth;
    const left = MARGIN + inset;
    const edge = this.rightEdge - inset + FRAME_MARGIN;
    this.widestFrame = Math.max(this.widestFrame, edge);

    const label = block.branches[0]?.label ?? "";

    this.frames.push({
      id: block.id,
      kind: block.kind,
      rect: { x: left, y: top, width: Math.max(edge - left, 0), height: this.y - top },
      ...(label ? { label } : {}),
      dividers,
    });
  }

  /**
   * Apoya cada flecha en el borde de la barra de ejecucion que toque.
   *
   * Sin esto una flecha entra hasta el centro de la linea de vida y atraviesa
   * la barra, que es justo lo que la barra quiere mostrar que no pasa.
   */
  private trimArrows() {
    for (const arrow of this.arrows) {
      const origin = innermost(this.bars, arrow.from, arrow.y);
      const destination = innermost(this.bars, arrow.to, arrow.y);

      if (arrow.from === arrow.to) {
        if (origin) {
          arrow.fromX = right(origin);
          arrow.toX = right(origin);
        }
        continue;
      }

      const rightward = arrow.fromX < arrow.toX;
      if (origin) arrow.fromX = rightward ? right(origin) : origin.x;
      if (destination) arrow.toX = rightward ? destination.x : right(destination);
    }
  }

  private finish(): LaneLayout {
    const laneBottom = this.y + TAIL;

    // Lo que quedo corriendo al acabar el guion se cierra al final. El orden
    // de recorrido no importa: las barras se ordenan justo despues, y dos no
    // pueden coincidir en `y` y `x` a la vez.
    for (const stack of this.stacks.values()) {
      for (const open of stack) this.pushBar(open, laneBottom);
    }
    this.stacks.clear();

    this.bars.sort((a, b) => a.rect.y - b.rect.y || a.rect.x - b.rect.x);
    this.trimArrows();

    const lanes: Lane[] = this.participants.map((node, i) => {
      const width = Math.max(headerSize(this.measures, node).width, MIN_HEADER_WIDTH);
      const x = this.xs[i]!;

      // Un participante creado a mitad del diagrama no tiene cabecera
      // arriba: nace donde le llega el mensaje.
      const birth = this.born.get(node);
      const headerY = birth === undefined ? MARGIN : birth - this.headerHeight / 2;
      const death = this.died.get(node);

      return {
        node,
        x,
        header: { x: x - width / 2, y: headerY, width, height: this.headerHeight },
        top: headerY + this.headerHeight,
        bottom: death ?? laneBottom,
        ...(death === undefined ? {} : { destroyed: true }),
      };
    });

    const edge = lanes.reduce(
      (acc, lane) => Math.max(acc, right(lane.header)),
      this.widestFrame,
    );

    return {
      lanes,
      arrows: this.arrows,
      bars: this.bars,
      frames: this.frames,
      notes: this.notes,
      size: { width: edge + MARGIN, height: laneBottom + MARGIN },
    };
  }
}

/** Carril de un participante, por su id de nodo. */
export function laneOf(layout: LaneLayout, node: string): Lane | undefined {
  return layout.lanes.find((lane) => lane.node === node);
}
