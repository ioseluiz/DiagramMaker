import { useCallback } from "react";

import type { DiagramKind } from "@/model/generated/DiagramKind";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { pickOpenPath, pickSavePath } from "./files";

/**
 * Comandos de documento.
 *
 * Los dialogos de archivo viven aqui y no en el store: el store debe poder
 * probarse sin tocar el sistema de archivos ni el plugin de Tauri.
 *
 * De los atajos ya no sabe nada: los declara el registro de comandos y los
 * dispara `useShortcuts`, que es el unico oyente de teclado que queda.
 */
export function useDocumentCommands() {
  const doc = useDocumentStore(selectDocument);
  const path = useDocumentStore((s) => s.path);
  const newDocument = useDocumentStore((s) => s.newDocument);
  const open = useDocumentStore((s) => s.open);
  const save = useDocumentStore((s) => s.save);

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

  return { createNew, openFile, saveFile, saveFileAs };
}
