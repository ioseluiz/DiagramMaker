import { create } from "zustand";

import type { Diagnostic } from "@/model/generated/Diagnostic";
import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { Document } from "@/model/generated/Document";
import type { RecentFile } from "@/model/generated/RecentFile";
import type { Recovery } from "@/model/generated/Recovery";
import type { Report } from "@/model/generated/Report";
import * as api from "@/model/api";
import {
  canRedo,
  canUndo,
  commit,
  commitCoalescing,
  initHistory,
  redo,
  undo,
  type History,
} from "./history";

/** Un cambio de seleccion tal y como lo emite el lienzo. */
export interface SelectionChange {
  id: string;
  selected: boolean;
}

export interface DocumentState {
  history: History<Document> | null;
  /** Ruta en disco. `null` = documento nuevo sin guardar. */
  path: string | null;
  /** Hay cambios sin guardar. */
  dirty: boolean;
  /** Ids de nodos y aristas seleccionados. */
  selection: string[];
  report: Report | null;
  /**
   * Diagnosticos de la ultima importacion.
   *
   * Se guardan aparte del informe de validacion porque hablan de otra cosa:
   * el informe dice que le pasa al diagrama de ahora, y estos dicen que se
   * perdio por el camino al leer el codigo. Duran hasta que llegue otro
   * documento, que es cuando dejan de venir a cuento.
   */
  imported: Diagnostic[];
  busy: boolean;
  error: string | null;
  recents: RecentFile[];
  /**
   * Copia de recuperacion encontrada al arrancar. Se ofrece al usuario; no se
   * restaura sola, porque el documento en pantalla podria ser mas reciente.
   */
  recovery: Recovery | null;
}

export interface DocumentActions {
  /**
   * Unico camino de mutacion del documento. Todo lo demas pasa por aqui, de
   * modo que no existe forma de cambiar el documento sin dejar rastro en el
   * historial.
   *
   * `coalesce` fusiona con el paso anterior si comparte etiqueta: es lo que
   * convierte un arrastre de cien eventos en un solo paso de deshacer.
   */
  edit(label: string, recipe: (draft: Document) => void, coalesce?: boolean): void;

  newDocument(kind: DiagramKind): Promise<void>;
  /**
   * Sustituye el documento por otro, descartando el historial. Es lo que
   * ocurre al importar: deshacer hasta antes de la importacion volveria a un
   * documento que ya no tiene nada que ver.
   */
  replaceDocument(document: Document, imported?: Diagnostic[]): void;
  open(path: string): Promise<void>;
  save(path?: string): Promise<void>;
  validate(): Promise<void>;

  undo(): void;
  redo(): void;
  select(ids: string[]): void;
  applySelection(changes: readonly SelectionChange[]): void;
  clearError(): void;

  loadRecents(): Promise<void>;
  forgetRecent(path: string): Promise<void>;
  /** Vuelca el documento actual a la copia de recuperacion. */
  autosave(): Promise<void>;
  checkRecovery(): Promise<void>;
  restoreRecovery(): void;
  /** Borra la copia del disco. */
  discardRecovery(): Promise<void>;
  /** Solo esconde el aviso; la copia sigue ahi. */
  dismissRecovery(): void;
}

export type DocumentStore = DocumentState & DocumentActions;

const EMPTY: DocumentState = {
  history: null,
  path: null,
  dirty: false,
  selection: [],
  report: null,
  imported: [],
  busy: false,
  error: null,
  recents: [],
  recovery: null,
};

/** Campos que se reinician al cargar otro documento. */
const FRESH = {
  path: null,
  dirty: false,
  selection: [] as string[],
  report: null,
  imported: [] as Diagnostic[],
  error: null,
};

function describe(error: unknown): string {
  return api.isAppError(error) ? error.message : String(error);
}

export const useDocumentStore = create<DocumentStore>()((set, get) => ({
  ...EMPTY,

  edit(label, recipe, coalesce = false) {
    const { history } = get();
    if (!history) return;

    const apply = coalesce ? commitCoalescing : commit;
    const next = apply(history, label, recipe);

    // Receta sin efecto: ni se marca sucio ni se gasta un paso de deshacer.
    if (next === history) return;

    set({ history: next, dirty: true, report: null });
  },

  async newDocument(kind) {
    set({ busy: true, error: null });
    try {
      const doc = await api.documentNew(kind, Date.now());
      set({ ...FRESH, history: initHistory(doc) });
    } catch (error) {
      set({ error: describe(error) });
    } finally {
      set({ busy: false });
    }
  },

  replaceDocument(document, imported = []) {
    set({ ...FRESH, history: initHistory(document), dirty: true, imported });
  },

  async open(path) {
    set({ busy: true, error: null });
    try {
      const doc = await api.documentOpen(path);
      set({ ...FRESH, history: initHistory(doc), path });
      set({ recents: await api.recentRemember(path) });
    } catch (error) {
      set({ error: describe(error) });
    } finally {
      set({ busy: false });
    }
  },

  async save(path) {
    const { history, path: current } = get();
    const target = path ?? current;
    if (!history || !target) return;

    set({ busy: true, error: null });
    try {
      const doc = {
        ...history.present,
        meta: { ...history.present.meta, modified: Date.now() },
      };
      await api.documentSave(target, doc);
      set({
        history: { ...history, present: doc },
        path: target,
        dirty: false,
        recents: await api.recentRemember(target),
      });
      // Lo guardado ya no necesita recuperarse.
      await api.autosaveClear();
    } catch (error) {
      set({ error: describe(error) });
    } finally {
      set({ busy: false });
    }
  },

  async validate() {
    const { history } = get();
    if (!history) return;

    try {
      set({ report: await api.documentValidate(history.present) });
    } catch (error) {
      set({ error: describe(error) });
    }
  },

  undo() {
    const { history } = get();
    if (!history || !canUndo(history)) return;
    set({ history: undo(history), dirty: true, report: null });
  },

  redo() {
    const { history } = get();
    if (!history || !canRedo(history)) return;
    set({ history: redo(history), dirty: true, report: null });
  },

  /**
   * React Flow emite su evento de seleccion en cada repintado, no solo
   * cuando cambia. Escribir un array nuevo cada vez cambiaba la identidad de
   * `selection`, lo que recalculaba todos los nodos y aristas del lienzo y
   * disparaba otro repintado: un bucle que se nota como latencia al pulsar.
   */
  select(ids) {
    const current = get().selection;
    if (
      current.length === ids.length &&
      current.every((id, index) => id === ids[index])
    ) {
      return;
    }
    set({ selection: ids });
  },

  /**
   * Aplica los cambios de seleccion que manda el lienzo.
   *
   * En modo controlado React Flow **no se guarda la seleccion**: la emite
   * como cambios y da por hecho que la lista que le devolvemos la llevara.
   * Ignorarlos dejaba el clic sin efecto, ni en nodos ni en relaciones. Con
   * las cajas aun se podia disimular encerrando una en un rectangulo de
   * seleccion, que va por otro camino; con una relacion no, porque el
   * rectangulo solo coge nodos. De ahi que se notara primero en las lineas.
   *
   * Un mismo gesto llega en dos llamadas, una de nodos y otra de aristas, asi
   * que esto parte de la seleccion que haya **en ese momento** y no de la del
   * render: con la del render, la segunda llamada pisaria a la primera y un
   * clic en una relacion acabaria sin seleccionar nada.
   */
  applySelection(changes) {
    if (changes.length === 0) return;

    const next = new Set(get().selection);
    for (const { id, selected } of changes) {
      if (selected) next.add(id);
      else next.delete(id);
    }

    get().select([...next]);
  },

  clearError() {
    set({ error: null });
  },

  async loadRecents() {
    try {
      set({ recents: await api.recentList() });
    } catch (error) {
      set({ error: describe(error) });
    }
  },

  async forgetRecent(path) {
    try {
      set({ recents: await api.recentForget(path) });
    } catch (error) {
      set({ error: describe(error) });
    }
  },

  async autosave() {
    const { history, path, dirty } = get();
    if (!history || !dirty) return;

    try {
      await api.autosaveWrite(history.present, path, Date.now());
    } catch {
      // Una copia de recuperacion que falla no debe interrumpir la edicion ni
      // ensuciar la barra de estado; el usuario no pidio esta operacion.
    }
  },

  async checkRecovery() {
    try {
      set({ recovery: await api.autosaveRead() });
    } catch {
      // Sin copia utilizable simplemente no se ofrece recuperar nada.
    }
  },

  restoreRecovery() {
    const { recovery } = get();
    if (!recovery) return;

    set({
      ...FRESH,
      history: initHistory(recovery.document),
      path: recovery.path ?? null,
      // Recuperado no es guardado: sigue habiendo cambios sin escribir a disco.
      dirty: true,
      recovery: null,
    });
  },

  async discardRecovery() {
    set({ recovery: null });
    try {
      await api.autosaveClear();
    } catch (error) {
      set({ error: describe(error) });
    }
  },

  dismissRecovery() {
    set({ recovery: null });
  },
}));

/* Selectores. Mantenerlos aqui evita que cada componente derive lo mismo. */

export const selectDocument = (s: DocumentStore): Document | null =>
  s.history?.present ?? null;

export const selectCanUndo = (s: DocumentStore): boolean =>
  s.history ? canUndo(s.history) : false;

export const selectCanRedo = (s: DocumentStore): boolean =>
  s.history ? canRedo(s.history) : false;
