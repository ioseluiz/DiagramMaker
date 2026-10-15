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
  const ctx = measureContext();
  if (!ctx) {
    // Sin canvas (jsdom, por ejemplo) una estimacion basta: lo unico que
    // depende de esto es el ancho, y no hay nada que dibujar.
    return text.length * size * 0.55;
  }
  ctx.font = fontString(size, bold, italic);
  return ctx.measureText(text).width;
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

export function placementOf(shape: NodeShape): LabelPlacement {
  const placement = labelPlacement(shape);
  return placement === "below" ? "below" : placement === "tab" ? "tab" : "center";
}

/** Tamano de fuente de cada emplazamiento: la solapa del paquete va menor. */
export function fontSizeFor(shape: NodeShape): number {
  return labelPlacement(shape) === "tab" ? 12 : FONT_SIZE;
}
