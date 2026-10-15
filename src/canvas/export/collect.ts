import type { Document } from "@/model/generated/Document";
import type { EdgePath } from "@/model/generated/EdgePath";
import type { ExportOptions } from "@/model/generated/ExportOptions";
import type { LabelLayout } from "@/model/generated/LabelLayout";
import type { RenderInput } from "@/model/generated/RenderInput";
import type { RenderTheme } from "@/model/generated/RenderTheme";
import { defaultSizeFor } from "@/model/shapes";
import { edgeStyle, nodeStyle } from "@/model/style";
import { hopsFor } from "@/canvas/graph/adapter";
import type { Hop } from "@/canvas/graph/crossings";
import {
  LINE_HEIGHT,
  diagramFontFamily,
  fontSizeFor,
  labelMaxWidth,
  placementOf,
  wrapText,
} from "@/canvas/text";

/**
 * Recoge todo lo que Rust necesita para exportar y que solo el navegador sabe.
 *
 * Dos cosas no se recalculan en Rust y salen de aqui:
 *
 * 1. **Las lineas de texto medidas.** El reparto lo hace `@/canvas/text`, el
 *    mismo modulo que usa el lienzo para pintar, asi que el SVG exportado
 *    rompe el texto exactamente por donde lo rompe la pantalla.
 * 2. **El trazado de cada arista.** Lo calcula React Flow con su propio
 *    algoritmo de ruteo; reimplementarlo en Rust daria curvas parecidas pero
 *    no iguales. Se lee del SVG que el lienzo ya dibujo, que es por definicion
 *    lo que el usuario esta viendo.
 * 3. **Los puentes de los cruces.** Dependen de por donde pasan todas las
 *    demas aristas, cosa que solo sabe el adaptador del lienzo.
 *
 * Los colores tambien se resuelven aqui: el lienzo pinta con variables CSS y
 * un SVG exportado que dependiera de ellas saldria en blanco fuera de la app.
 */

/** Variable CSS resuelta a un valor concreto. */
function cssVar(name: string, fallback: string): string {
  const value = getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  return value || fallback;
}

export function resolveTheme(): RenderTheme {
  return {
    background: cssVar("--canvas-bg", "#ffffff"),
    nodeFill: cssVar("--surface-0", "#ffffff"),
    nodeStroke: cssVar("--border-strong", "#b6bec9"),
    nodeText: cssVar("--text", "#16191d"),
    edgeStroke: cssVar("--border-strong", "#b6bec9"),
    edgeText: cssVar("--text-muted", "#5b6471"),
    fontFamily: diagramFontFamily(),
  };
}

function nodeLabels(doc: Document): Record<string, LabelLayout> {
  const labels: Record<string, LabelLayout> = {};

  for (const node of doc.nodes) {
    if (!node.label.trim()) continue;

    const style = nodeStyle(node);
    const [defaultWidth] = defaultSizeFor(node.shape);
    const width = node.geometry?.width ?? defaultWidth;
    const fontSize = style.fontSize ?? fontSizeFor(node.shape);

    const lines = wrapText(node.label, {
      maxWidth: labelMaxWidth(node.shape, width),
      size: fontSize,
      bold: style.bold,
      italic: style.italic,
    });
    if (lines.length === 0) continue;

    labels[node.id] = {
      lines,
      fontSize,
      lineHeight: LINE_HEIGHT,
      placement: placementOf(node.shape),
      color: style.textColor,
      bold: style.bold ?? false,
      italic: style.italic ?? false,
    };
  }

  return labels;
}

/** Ancho maximo de la etiqueta de una arista antes de partirla. */
const EDGE_LABEL_MAX_WIDTH = 180;
const EDGE_FONT_SIZE = 11;
const EDGE_LINE_HEIGHT = 14;

function edgeLabels(doc: Document): Record<string, LabelLayout> {
  const labels: Record<string, LabelLayout> = {};

  for (const edge of doc.edges) {
    const text = edge.label?.trim();
    if (!text) continue;

    const style = edgeStyle(edge);
    const fontSize = style.fontSize ?? EDGE_FONT_SIZE;
    const lines = wrapText(text, { maxWidth: EDGE_LABEL_MAX_WIDTH, size: fontSize });
    if (lines.length === 0) continue;

    labels[edge.id] = {
      lines,
      fontSize,
      lineHeight: EDGE_LINE_HEIGHT,
      placement: "center",
      color: style.textColor,
      bold: false,
      italic: false,
    };
  }

  return labels;
}

/**
 * Lee de la pantalla el trazado que React Flow dibujo para cada arista.
 *
 * El `d` esta en coordenadas del diagrama (React Flow aplica el zoom con una
 * transformacion sobre el contenedor, no sobre los trazados), asi que se puede
 * mandar a Rust tal cual.
 */
function edgePaths(
  doc: Document,
  root: ParentNode = document,
  hops: ReadonlyMap<string, readonly Hop[]> = new Map(),
): Record<string, EdgePath> {
  const paths: Record<string, EdgePath> = {};

  for (const edge of doc.edges) {
    const selector = `.react-flow__edge[data-id="${CSS.escape(edge.id)}"] path.react-flow__edge-path`;
    const element = root.querySelector<SVGPathElement>(selector);

    const d = element?.getAttribute("d");
    if (!element || !d) continue;

    paths[edge.id] = {
      d,
      labelAt: midpointOf(element) ?? undefined,
      hops: [...(hops.get(edge.id) ?? [])],
    };
  }

  return paths;
}

/** Punto medio del trazado, medido sobre el propio elemento SVG. */
function midpointOf(path: SVGPathElement): { x: number; y: number } | null {
  try {
    const length = path.getTotalLength();
    if (!Number.isFinite(length) || length === 0) return null;

    const point = path.getPointAtLength(length / 2);
    return { x: point.x, y: point.y };
  } catch {
    // `getTotalLength` no existe en jsdom y puede fallar con trazados raros.
    return null;
  }
}

export const DEFAULT_EXPORT_OPTIONS: ExportOptions = {
  padding: 24,
  opaqueBackground: true,
  scale: 2,
};

export function collectRenderInput(
  document: Document,
  options: ExportOptions = DEFAULT_EXPORT_OPTIONS,
  root?: ParentNode,
): RenderInput {
  return {
    document,
    labels: { ...nodeLabels(document), ...edgeLabels(document) },
    edges: edgePaths(document, root, hopsFor(document)),
    theme: resolveTheme(),
    options,
    // Solo las usa el reparto de una secuencia, y de momento ese tipo de
    // diagrama no se puede dibujar: las rellenara el lienzo de carriles.
    measures: { headers: {}, texts: {} },
  };
}

export const __testing = { nodeLabels, edgeLabels, edgePaths };
