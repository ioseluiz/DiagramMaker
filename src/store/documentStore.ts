import { create } from "zustand";

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

export interface DocumentState {
  history: History<Document> | null;
  /** Ruta en disco. `null` = documento nuevo sin guardar. */
  path: string | null;
  /** Hay cambios sin guardar. */
  dirty: boolean;
  /** Ids de nodos y aristas seleccionados. */
  selection: string[];
  report: Report | null;
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
  open(path: string): Promise<void>;
  save(path?: string): Promise<void>;
  validate(): Promise<void>;

  undo(): void;
  redo(): void;
  select(ids: string[]): void;
  clearError(): void;

  loadRecents(): Promise<void>;
  forgetRecent(path: string): Promise<void>;
  /** Vuelca el documento actual a la copia de recuperacion. */
  autosave(): Promise<void>;
  checkRecovery(): Promise<void>;
  restoreRecovery(): void;
  discardRecovery(): Promise<void>;
}

export type DocumentStore = DocumentState & DocumentActions;

const EMPTY: DocumentState = {
  history: null,
  path: null,
  dirty: false,
  selection: [],
  report: null,
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

  select(ids) {
    set({ selection: ids });
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
}));

/* Selectores. Mantenerlos aqui evita que cada componente derive lo mismo. */

export const selectDocument = (s: DocumentStore): Document | null =>
  s.history?.present ?? null;

export const selectCanUndo = (s: DocumentStore): boolean =>
  s.history ? canUndo(s.history) : false;

export const selectCanRedo = (s: DocumentStore): boolean =>
  s.history ? canRedo(s.history) : false;
