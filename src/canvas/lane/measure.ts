import type { Document } from "@/model/generated/Document";
import type { LabelLayout } from "@/model/generated/LabelLayout";
import type { Measures } from "@/model/generated/Measures";
import { HEADER_HEIGHT, walkSteps } from "@/model/laneGeometry";
import { dividerCaption, frameCaption, scriptOf } from "@/model/sequence";
import { LINE_HEIGHT, measureWidth, wrapText } from "@/canvas/text";

/**
 * Medidas y etiquetas de un diagrama de secuencia.
 *
 * El reparto (`laneGeometry`) no mide texto: recibe anchos. Medirlos es cosa
 * del navegador y se hace aqui, una sola vez, para que el lienzo y la
 * exportacion partan exactamente de los mismos numeros. Si cada uno midiera
 * por su cuenta, el PNG no se pareceria a la pantalla.
 */

export const HEADER_FONT_SIZE = 13;
export const MESSAGE_FONT_SIZE = 11;
export const MESSAGE_LINE_HEIGHT = 14;

/** Aire a cada lado del nombre dentro de la caja de un participante. */
const HEADER_PADDING = 14;
/** A partir de aqui el texto de un mensaje se parte en varias lineas. */
const MESSAGE_MAX_WIDTH = 220;
/** Y el de una nota, que suele ser mas largo. */
const NOTE_MAX_WIDTH = 200;

/** Tamano de un bloque de lineas ya medidas. */
function sizeOf(lines: { width: number }[], lineHeight: number) {
  return {
    width: lines.reduce((acc, line) => Math.max(acc, line.width), 0),
    height: lines.length * lineHeight,
  };
}

/**
 * Lo que el reparto necesita: el ancho de cada cabecera y el de cada texto.
 *
 * Solo entran los textos que empujan la geometria —mensajes y notas—. El
 * rotulo de un marco no ensancha nada: el marco ocupa el diagrama entero.
 */
export function laneMeasures(doc: Document): Measures {
  const headers: Measures["headers"] = {};
  const texts: Measures["texts"] = {};

  for (const node of doc.nodes) {
    headers[node.id] = {
      width: measureWidth(node.label, HEADER_FONT_SIZE) + HEADER_PADDING * 2,
      height: HEADER_HEIGHT,
    };
  }

  for (const step of walkSteps(scriptOf(doc).steps)) {
    if (step.type === "message") {
      const text = step.text?.trim();
      if (!text) continue;
      const lines = wrapText(text, {
        maxWidth: MESSAGE_MAX_WIDTH,
        size: MESSAGE_FONT_SIZE,
      });
      texts[step.id] = sizeOf(lines, MESSAGE_LINE_HEIGHT);
    } else if (step.type === "note") {
      const lines = wrapText(step.text, {
        maxWidth: NOTE_MAX_WIDTH,
        size: MESSAGE_FONT_SIZE,
      });
      texts[step.id] = sizeOf(lines, MESSAGE_LINE_HEIGHT);
    }
  }

  return { headers, texts };
}

function labelOf(
  text: string,
  fontSize: number,
  lineHeight: number,
  maxWidth: number,
): LabelLayout | undefined {
  const lines = wrapText(text, { maxWidth, size: fontSize });
  if (lines.length === 0) return undefined;

  return {
    lines,
    fontSize,
    lineHeight,
    placement: "center",
    bold: false,
    italic: false,
  };
}

/**
 * Las etiquetas que el exportador de Rust dibuja, por id de la pieza: el nodo
 * para la cabecera de un participante, el paso para un mensaje o una nota, el
 * bloque para el rotulo del marco y la rama para el de un separador.
 */
export function laneLabels(doc: Document): Record<string, LabelLayout> {
  const labels: Record<string, LabelLayout> = {};

  const put = (id: string, label: LabelLayout | undefined) => {
    if (label) labels[id] = label;
  };

  for (const node of doc.nodes) {
    put(
      node.id,
      labelOf(node.label, HEADER_FONT_SIZE, LINE_HEIGHT, Number.POSITIVE_INFINITY),
    );
  }

  for (const step of walkSteps(scriptOf(doc).steps)) {
    if (step.type === "message") {
      const text = step.text?.trim();
      if (text) {
        put(
          step.id,
          labelOf(text, MESSAGE_FONT_SIZE, MESSAGE_LINE_HEIGHT, MESSAGE_MAX_WIDTH),
        );
      }
    } else if (step.type === "note") {
      put(
        step.id,
        labelOf(step.text, MESSAGE_FONT_SIZE, MESSAGE_LINE_HEIGHT, NOTE_MAX_WIDTH),
      );
    } else {
      const caption = frameCaption(step.kind, step.branches[0]?.label ?? "");
      put(
        step.id,
        labelOf(
          caption,
          MESSAGE_FONT_SIZE,
          MESSAGE_LINE_HEIGHT,
          Number.POSITIVE_INFINITY,
        ),
      );

      // La primera rama se rotula en la esquina del marco; las demas, en su
      // separador.
      for (const branch of step.branches.slice(1)) {
        put(
          branch.id,
          labelOf(
            dividerCaption(step.kind, branch.label ?? ""),
            MESSAGE_FONT_SIZE,
            MESSAGE_LINE_HEIGHT,
            Number.POSITIVE_INFINITY,
          ),
        );
      }
    }
  }

  return labels;
}
