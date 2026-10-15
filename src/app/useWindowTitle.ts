import { useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { basename } from "./time";
import { useAppInfo } from "./useAppInfo";

/**
 * Mantiene el titulo de la ventana al dia.
 *
 * Lleva el documento abierto delante y el nombre de la aplicacion con su
 * version detras, que es el orden habitual: en la barra de tareas lo que
 * interesa distinguir es el archivo, no el programa.
 *
 * El punto de los cambios sin guardar va al principio para que se vea aunque
 * el titulo quede recortado.
 */
export function useWindowTitle(): void {
  const doc = useDocumentStore(selectDocument);
  const path = useDocumentStore((s) => s.path);
  const dirty = useDocumentStore((s) => s.dirty);
  const info = useAppInfo();

  useEffect(() => {
    const app = info ? `DiagramMaker ${info.version}` : "DiagramMaker";
    const name = path ? basename(path) : doc?.meta.title;

    const title = name ? `${dirty ? "• " : ""}${name} — ${app}` : app;

    // Si la ventana no esta disponible no hay nada que arreglar: el titulo
    // se queda con el que trae de la configuracion.
    void getCurrentWindow()
      .setTitle(title)
      .catch(() => {});
  }, [dirty, doc?.meta.title, info, path]);
}
