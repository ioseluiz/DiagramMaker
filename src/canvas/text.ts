import type { LabelPlacement } from "@/model/generated/LabelPlacement";
import type { NodeShape } from "@/model/generated/NodeShape";
import type { TextLine } from "@/model/generated/TextLine";
import { labelPlacement } from "@/model/shapes";

/**
 * Medicion y partido de texto.
 *
 * Es la pieza que cierra el riesgo principal del proyecto: el navegador y el
 * rasterizador de Rust no componen el texto igual, asi que **solo se mide
 * aqui**. El lienzo pinta las lineas que devuelve este modulo y la exportacion
 * manda esas mismas lineas, con su ancho, a Rust. Rust nunca vuelve a medir:
 * se limita a colocar lo que recibe.
 *
 * Si el lienzo dejara que CSS partiera el texto por su cuenta, la exportacion
 * tendria que adivinar donde cayeron los saltos — y acabaria adivinando mal.
 */

export const FONT_SIZE = 13;
export const LINE_HEIGHT = 17;
/** Margen interior del texto dentro de la caja del nodo. */
export const TEXT_PADDING = 10;
/** Ancho maximo de la etiqueta que cuelga bajo un actor. */
const BELOW_MAX_WIDTH = 160;
/** Mas lineas que esto y la etiqueta se corta con puntos suspensivos. */
const MAX_LINES = 6;

let context: CanvasRenderingContext2D | null = null;

function measureContext(): CanvasRenderingContext2D | null {
  if (context) return context;
  context = document.createElement("canvas").getContext("2d");
  return context;
}

/**
 * Cache de anchos ya medidos.
 *
 * El lienzo mide el mismo texto una y otra vez: cada repintado de una tabla
 * pregunta por el nombre y el tipo de todas sus columnas. `measureText` no es
 * gratis, y con una decena de tablas eso son cientos de llamadas por
 * fotograma.
 *
 * La clave no incluye la familia tipografica porque `--font-diagram` es fija
 * y no cambia con el tema. El dia que se pueda elegir, la cache hay que
 * vaciarla al cambiarla o devolvera medidas de la anterior.
 */
const widths = new Map<string, number>();

/** Mas alla de esto la cache se vacia; un diagrama no tiene tanto texto. */
const CACHE_LIMIT = 4_000;

/** Familia tipografica efectiva del diagrama, resuelta desde el tema. */
export function diagramFontFamily(): string {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue("--font-diagram")
    .trim();
  return value || "system-ui, sans-serif";
}

function fontString(size: number, bold: boolean, italic: boolean): string {
  const style = italic ? "italic " : "";
  const weight = bold ? "600" : "400";
  return `${style}${weight} ${size}px ${diagramFontFamily()}`;
}

export function measureWidth(
  text: string,
  size = FONT_SIZE,
  bold = false,
  italic = false,
) {
  if (text === "") return 0;

  const key = `${size}|${bold ? 1 : 0}${italic ? 1 : 0}|${text}`;
  const cached = widths.get(key);
  if (cached !== undefined) return cached;

  const ctx = measureContext();
  // Sin canvas (jsdom, por ejemplo) una estimacion basta: lo unico que
  // depende de esto es el ancho, y no hay nada que dibujar.
  const width = ctx
    ? ((ctx.font = fontString(size, bold, italic)), ctx.measureText(text).width)
    : text.length * size * 0.55;

  if (widths.size >= CACHE_LIMIT) widths.clear();
  widths.set(key, width);

  return width;
}

export interface WrapOptions {
  maxWidth: number;
  size?: number;
  bold?: boolean;
  italic?: boolean;
}

/**
 * Parte el texto en lineas que caben en `maxWidth`.
 *
 * Rompe por espacios; una palabra mas ancha que la caja se parte por
 * caracteres, porque dejarla salirse seria peor. Respeta los saltos de linea
 * explicitos del usuario.
 */
export function wrapText(text: string, options: WrapOptions): TextLine[] {
  const { maxWidth, size = FONT_SIZE, bold = false, italic = false } = options;
  const width = (s: string) => measureWidth(s, size, bold, italic);

  if (!text) return [];

  const lines: TextLine[] = [];

  for (const paragraph of text.split("\n")) {
    if (paragraph.trim() === "") {
      lines.push({ text: "", width: 0 });
      continue;
    }

    let current = "";
    for (const word of paragraph.split(/\s+/).filter(Boolean)) {
      const candidate = current ? `${current} ${word}` : word;

      if (width(candidate) <= maxWidth || !current) {
        // Una palabra sola mas ancha que la caja se parte por caracteres.
        if (!current && width(word) > maxWidth) {
          for (const piece of breakWord(word, maxWidth, width)) {
            lines.push({ text: piece, width: width(piece) });
          }
          current = lines.pop()?.text ?? "";
          continue;
        }
        current = candidate;
      } else {
        lines.push({ text: current, width: width(current) });
        current = word;
      }
    }

    if (current) lines.push({ text: current, width: width(current) });
  }

  return clamp(lines, width);
}

function breakWord(
  word: string,
  maxWidth: number,
  width: (s: string) => number,
): string[] {
  const pieces: string[] = [];
  let current = "";

  for (const char of word) {
    if (current && width(current + char) > maxWidth) {
      pieces.push(current);
      current = char;
    } else {
      current += char;
    }
  }

  if (current) pieces.push(current);
  return pieces;
}

/** Corta a `MAX_LINES` y marca el recorte con puntos suspensivos. */
function clamp(lines: TextLine[], width: (s: string) => number): TextLine[] {
  if (lines.length <= MAX_LINES) return lines;

  const kept = lines.slice(0, MAX_LINES);
  const last = kept[MAX_LINES - 1]!;
  const text = `${last.text.replace(/\s+\S*$/, "")}…`;
  kept[MAX_LINES - 1] = { text, width: width(text) };

  return kept;
}

/**
 * Ancho disponible para la etiqueta de un nodo, segun donde se coloque.
 */
export function labelMaxWidth(shape: NodeShape, width: number): number {
  switch (labelPlacement(shape)) {
    case "below":
      return BELOW_MAX_WIDTH;
    case "tab":
      return Math.min(width * 0.45, 120) - TEXT_PADDING;
    default:
      return Math.max(width - TEXT_PADDING * 2, 20);
  }
}

/**
 * Alto de linea para un cuerpo de letra dado.
 *
 * Proporcional, no fijo. Con un alto constante, subir el cuerpo del texto
 * amontona unas lineas sobre otras y bajarlo las separa de mas; y como el
 * lienzo y el exportador usan los dos esta funcion, rompen igual o bien los
 * dos a la vez. La proporcion es la que ya habia entre los valores por
 * defecto, asi que al tamano de siempre sale el alto de siempre.
 */
export function lineHeightFor(size: number): number {
  return Math.round((size * LINE_HEIGHT) / FONT_SIZE);
}

/**
 * Caja que pide una etiqueta escrita con un cuerpo de letra dado.
 *
 * Mide sin limite de ancho, respetando solo los saltos de linea que el autor
 * escribio: sirve para que un nodo crezca con su texto cuando se le cambia
 * el tamano de la letra, y para eso hay que saber cuanto ocuparia el texto
 * **sin** que la caja actual lo obligue a partirse.
 */
export function boxForLabel(
  label: string,
  size: number,
  options: { bold?: boolean; italic?: boolean } = {},
): { width: number; height: number } {
  const lines = wrapText(label, { maxWidth: Infinity, size, ...options });
  const widest = lines.reduce((max, line) => Math.max(max, line.width), 0);

  return {
    width: Math.ceil(widest + TEXT_PADDING * 2),
    height: Math.ceil(lines.length * lineHeightFor(size) + TEXT_PADDING),
  };
}

export function placementOf(shape: NodeShape): LabelPlacement {
  const placement = labelPlacement(shape);
  return placement === "below" ? "below" : placement === "tab" ? "tab" : "center";
}

/** Tamano de fuente de cada emplazamiento: la solapa del paquete va menor. */
export function fontSizeFor(shape: NodeShape): number {
  return labelPlacement(shape) === "tab" ? 12 : FONT_SIZE;
}
