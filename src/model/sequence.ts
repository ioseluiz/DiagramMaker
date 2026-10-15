import type { BlockKind } from "./generated/BlockKind";
import type { Document } from "./generated/Document";
import type { MessageKind } from "./generated/MessageKind";
import type { NotePlacement } from "./generated/NotePlacement";
import type { SequenceScript } from "./generated/SequenceScript";
import type { Step } from "./generated/Step";

/**
 * Lo que el guion de una secuencia necesita saber de si mismo y el modelo
 * generado no dice: como se rotula cada cosa y como se busca un paso dentro
 * del arbol.
 *
 * Los rotulos son espejo de `BlockKind::tag` y `BlockKind::divider_tag` de
 * Rust. Son palabras de la notacion UML, no traducciones: un `alt` se rotula
 * `alt` en cualquier idioma.
 */

export const BLOCK_TAG: Record<BlockKind, string> = {
  alt: "alt",
  opt: "opt",
  loop: "loop",
  par: "par",
  critical: "critical",
  break: "break",
  group: "group",
};

/** Como se llama la rama que no es la primera. */
export const DIVIDER_TAG: Record<BlockKind, string> = {
  alt: "else",
  par: "and",
  opt: "else",
  loop: "else",
  critical: "else",
  break: "else",
  group: "else",
};

/** Rotulo de la esquina de un marco: `alt [hay stock]`. */
export function frameCaption(kind: BlockKind, label: string): string {
  const condition = label.trim();
  return condition ? `${BLOCK_TAG[kind]} [${condition}]` : BLOCK_TAG[kind];
}

/** Rotulo de un separador entre ramas: `[no hay stock]`. */
export function dividerCaption(kind: BlockKind, label: string): string {
  const condition = label.trim();
  return condition ? `[${condition}]` : DIVIDER_TAG[kind];
}

/* --- Nombres para la interfaz -------------------------------------------- */

export const MESSAGE_KIND_LABEL: Record<MessageKind, string> = {
  sync: "Llamada",
  async: "Asincrono",
  reply: "Retorno",
  create: "Crea",
  destroy: "Destruye",
};

export const BLOCK_KIND_LABEL: Record<BlockKind, string> = {
  alt: "Alternativa (alt)",
  opt: "Opcional (opt)",
  loop: "Bucle (loop)",
  par: "En paralelo (par)",
  critical: "Critico (critical)",
  break: "Interrupcion (break)",
  group: "Grupo (group)",
};

export const NOTE_PLACEMENT_LABEL: Record<NotePlacement, string> = {
  leftOf: "A la izquierda",
  rightOf: "A la derecha",
  over: "Encima",
};

/* --- Recorrido ----------------------------------------------------------- */

export const EMPTY_SCRIPT: SequenceScript = { steps: [] };

/** Guion del documento, o uno vacio si no es una secuencia. */
export function scriptOf(doc: Document): SequenceScript {
  return doc.script ?? EMPTY_SCRIPT;
}

/** Un paso y donde vive: la lista que lo contiene y su posicion en ella. */
export interface Located {
  step: Step;
  siblings: Step[];
  position: number;
  /** Profundidad de anidamiento; 0 en la raiz del guion. */
  depth: number;
}

/**
 * Aplana el guion para recorrerlo o pintarlo como lista, conservando de cada
 * paso donde estaba. Es lo que permite reordenar y borrar sin tener que
 * volver a buscar por el arbol.
 */
export function flatten(steps: Step[], depth = 0): Located[] {
  const out: Located[] = [];

  steps.forEach((step, position) => {
    out.push({ step, siblings: steps, position, depth });
    if (step.type === "block") {
      for (const branch of step.branches) {
        out.push(...flatten(branch.steps, depth + 1));
      }
    }
  });

  return out;
}

/** Busca un paso por id, con su contexto. `undefined` si no esta. */
export function locate(steps: Step[], id: string): Located | undefined {
  return flatten(steps).find((entry) => entry.step.id === id);
}

/** Texto de una linea de la lista de pasos. */
export function describeStep(step: Step, nameOf: (id: string) => string): string {
  switch (step.type) {
    case "message": {
      const arrow = step.kind === "reply" ? "⇠" : "→";
      const text = step.text?.trim();
      return `${nameOf(step.from)} ${arrow} ${nameOf(step.to)}${text ? `: ${text}` : ""}`;
    }
    case "block":
      return frameCaption(step.kind, step.branches[0]?.label ?? "");
    case "note":
      return `nota: ${step.text}`;
  }
}
