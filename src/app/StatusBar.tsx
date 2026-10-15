import { useEffect } from "react";

import { selectDocument, useDocumentStore } from "@/store/documentStore";

/** Nombre del archivo a partir de su ruta completa. */
function basename(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export function StatusBar() {
  const doc = useDocumentStore(selectDocument);
  const path = useDocumentStore((s) => s.path);
  const dirty = useDocumentStore((s) => s.dirty);
  const busy = useDocumentStore((s) => s.busy);
  const report = useDocumentStore((s) => s.report);
  const error = useDocumentStore((s) => s.error);
  const clearError = useDocumentStore((s) => s.clearError);
  const validate = useDocumentStore((s) => s.validate);

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
    </footer>
  );
}
