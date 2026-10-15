import {
  applyPatches,
  enablePatches,
  produceWithPatches,
  type Objectish,
  type Patch,
} from "immer";

enablePatches();

/**
 * Una entrada del historial guarda los parches en ambos sentidos, que es lo
 * que `produceWithPatches` devuelve gratis. No hay que escribir el inverso de
 * cada operacion a mano: eso es justamente lo que vuelve frágiles los
 * command pattern caseros.
 */
export interface HistoryEntry {
  label: string;
  redo: Patch[];
  undo: Patch[];
}

export interface History<T extends Objectish> {
  present: T;
  past: HistoryEntry[];
  future: HistoryEntry[];
}

/** Numero maximo de pasos recordados. Mas alla de esto, el historial poda. */
export const HISTORY_LIMIT = 200;

export function initHistory<T extends Objectish>(present: T): History<T> {
  return { present, past: [], future: [] };
}

/**
 * Aplica `recipe` y registra el cambio.
 *
 * Si la receta no cambia nada, devuelve el historial **tal cual** (misma
 * referencia): un clic que no modifica el documento no debe gastar un paso de
 * deshacer.
 */
/**
 * Envuelve una receta para que lo que devuelva no llegue a immer.
 *
 * Immer trata el valor devuelto como el estado nuevo y lanza si ademas se
 * modifico el borrador. Muchas operaciones devuelven el id de lo que acaban
 * de crear, y escribir `edit("x", (d) => addMessage(d, ...))` es lo natural:
 * TypeScript no lo impide —devolver algo donde se espera `void` es legal— asi
 * que el descarte se hace aqui, una vez, en vez de confiar en que nadie se
 * despiste.
 */
function run<T>(recipe: (draft: T) => void): (draft: T) => void {
  return (draft) => {
    recipe(draft);
  };
}

export function commit<T extends Objectish>(
  history: History<T>,
  label: string,
  recipe: (draft: T) => void,
): History<T> {
  const [next, redo, undo] = produceWithPatches(history.present, run(recipe));

  if (redo.length === 0) return history;

  const past = [...history.past, { label, redo, undo }];

  return {
    present: next,
    past: past.length > HISTORY_LIMIT ? past.slice(past.length - HISTORY_LIMIT) : past,
    future: [],
  };
}

/**
 * Fusiona con el paso anterior si comparten etiqueta.
 *
 * Es lo que evita que arrastrar un nodo deje cien pasos de deshacer: todos los
 * `move:n1` consecutivos colapsan en uno. El resultado es un unico paso que va
 * del estado previo al arrastre al estado final.
 */
export function commitCoalescing<T extends Objectish>(
  history: History<T>,
  label: string,
  recipe: (draft: T) => void,
): History<T> {
  const next = commit(history, label, recipe);
  if (next === history) return history;

  const previous = history.past.at(-1);
  if (!previous || previous.label !== label) return next;

  const merged = next.past.slice(0, -2);
  const fresh = next.past.at(-1)!;

  merged.push({
    label,
    redo: [...previous.redo, ...fresh.redo],
    // El deshacer se recorre al reves: primero se revierte lo ultimo.
    undo: [...fresh.undo, ...previous.undo],
  });

  return { ...next, past: merged };
}

export function undo<T extends Objectish>(history: History<T>): History<T> {
  const entry = history.past.at(-1);
  if (!entry) return history;

  return {
    present: applyPatches(history.present, entry.undo),
    past: history.past.slice(0, -1),
    future: [entry, ...history.future],
  };
}

export function redo<T extends Objectish>(history: History<T>): History<T> {
  const [entry, ...rest] = history.future;
  if (!entry) return history;

  return {
    present: applyPatches(history.present, entry.redo),
    past: [...history.past, entry],
    future: rest,
  };
}

export function canUndo<T extends Objectish>(history: History<T>): boolean {
  return history.past.length > 0;
}

export function canRedo<T extends Objectish>(history: History<T>): boolean {
  return history.future.length > 0;
}
