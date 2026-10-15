import { useEffect } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { basename } from "./time";
import { useAppInfo } from "./useAppInfo";

const REPOSITORY = "https://github.com/ioseluiz/DiagramMaker";
const AUTHOR = "ioseluiz";

export function StatusBar() {
  const doc = useDocumentStore(selectDocument);
  const path = useDocumentStore((s) => s.path);
  const dirty = useDocumentStore((s) => s.dirty);
  const busy = useDocumentStore((s) => s.busy);
  const report = useDocumentStore((s) => s.report);
  const error = useDocumentStore((s) => s.error);
  const clearError = useDocumentStore((s) => s.clearError);
  const validate = useDocumentStore((s) => s.validate);
  const info = useAppInfo();

  // Revalidar cuando el documento cambia, con un respiro para no lanzar una
  // llamada por cada pixel de un arrastre.
  useEffect(() => {
    if (!doc) return;
    const timer = setTimeout(() => void validate(), 400);
    return () => clearTimeout(timer);
  }, [doc, validate]);

  const errors = report?.diagnostics.filter((d) => d.severity === "error") ?? [];

  return (
    <footer className="status-bar">
      <span className="status-bar__file">
        {path ? basename(path) : "Sin guardar"}
        {dirty && " •"}
      </span>

      {doc && (
        <span className="status-bar__counts">
          {doc.nodes.length} nodos · {doc.edges.length} relaciones
        </span>
      )}

      <span className="status-bar__spacer" />

      {busy && <span>Trabajando…</span>}

      {errors.length > 0 && (
        <span
          className="status-bar__problem"
          title={errors.map((d) => d.message).join("\n")}
        >
          {errors.length} {errors.length === 1 ? "problema" : "problemas"}
        </span>
      )}

      {error && (
        <button type="button" className="status-bar__error" onClick={clearError}>
          {error}
        </button>
      )}

      {info && (
        <span
          className="status-bar__version"
          title={`Esquema de documento v${info.schemaVersion}`}
        >
          v{info.version}
        </span>
      )}

      <button
        type="button"
        className="status-bar__repo"
        title={`Abrir ${REPOSITORY} en el navegador`}
        onClick={() => void openUrl(REPOSITORY)}
      >
        <GitHubMark />
        {AUTHOR}
      </button>
    </footer>
  );
}

/** Marca de GitHub, en la misma rejilla de 24x24 que el resto de iconos. */
function GitHubMark() {
  return (
    <svg viewBox="0 0 24 24" width="13" height="13" aria-hidden focusable="false">
      <path
        fill="currentColor"
        d="M12 2.2a10 10 0 0 0-3.16 19.49c.5.09.68-.22.68-.48l-.01-1.7c-2.78.6-3.37-1.34-3.37-1.34-.45-1.16-1.11-1.47-1.11-1.47-.91-.62.07-.61.07-.61 1 .07 1.53 1.03 1.53 1.03.9 1.53 2.36 1.09 2.94.83.09-.65.35-1.09.63-1.34-2.22-.25-4.56-1.11-4.56-4.95 0-1.09.39-1.99 1.03-2.69-.1-.25-.45-1.27.1-2.65 0 0 .84-.27 2.75 1.03a9.5 9.5 0 0 1 5 0c1.91-1.3 2.75-1.03 2.75-1.03.55 1.38.2 2.4.1 2.65.64.7 1.03 1.6 1.03 2.69 0 3.85-2.34 4.7-4.57 4.95.36.31.68.92.68 1.85l-.01 2.75c0 .26.18.58.69.48A10 10 0 0 0 12 2.2z"
      />
    </svg>
  );
}
