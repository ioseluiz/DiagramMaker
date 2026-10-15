import { useCallback, useEffect } from "react";

import type { DiagramKind } from "@/model/generated/DiagramKind";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { pickOpenPath, pickSavePath } from "./files";

/**
 * Comandos de documento y sus atajos.
 *
 * Los dialogos de archivo viven aqui y no en el store: el store debe poder
 * probarse sin tocar el sistema de archivos ni el plugin de Tauri.
 */
export function useDocumentCommands() {
  const doc = useDocumentStore(selectDocument);
  const path = useDocumentStore((s) => s.path);
  const newDocument = useDocumentStore((s) => s.newDocument);
  const open = useDocumentStore((s) => s.open);
  const save = useDocumentStore((s) => s.save);
  const undo = useDocumentStore((s) => s.undo);
  const redo = useDocumentStore((s) => s.redo);

  const createNew = useCallback(
    (kind: DiagramKind = "architecture") => void newDocument(kind),
    [newDocument],
  );

  const openFile = useCallback(async () => {
    const picked = await pickOpenPath();
    if (picked) await open(picked);
  }, [open]);

  const saveFile = useCallback(async () => {
    if (path) {
      await save();
      return;
    }
    const picked = await pickSavePath(doc?.meta.title ?? "diagrama");
    if (picked) await save(picked);
  }, [doc?.meta.title, path, save]);

  const saveFileAs = useCallback(async () => {
    const picked = await pickSavePath(doc?.meta.title ?? "diagrama");
    if (picked) await save(picked);
  }, [doc?.meta.title, save]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (!event.ctrlKey && !event.metaKey) return;

      // Dentro de un campo de texto mandan los atajos del propio campo.
      const target = event.target as HTMLElement | null;
      const editing =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target?.isContentEditable === true;

      const key = event.key.toLowerCase();

      if (key === "z" && !editing) {
        event.preventDefault();
        if (event.shiftKey) redo();
        else undo();
        return;
      }

      if (key === "y" && !editing) {
        event.preventDefault();
        redo();
        return;
      }

      if (key === "s") {
        event.preventDefault();
        void (event.shiftKey ? saveFileAs() : saveFile());
        return;
      }

      if (key === "o" && !editing) {
        event.preventDefault();
        void openFile();
        return;
      }

      if (key === "n" && !editing) {
        event.preventDefault();
        createNew();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [createNew, openFile, redo, saveFile, saveFileAs, undo]);

  return { createNew, openFile, saveFile, saveFileAs };
}
