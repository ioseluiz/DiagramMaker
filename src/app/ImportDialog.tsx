import { useEffect, useState } from "react";

import type { Diagnostic } from "@/model/generated/Diagnostic";
import type { SourceLanguage } from "@/model/generated/SourceLanguage";
import * as api from "@/model/api";
import { applyAutoLayout } from "@/layout/autoLayout";
import { useDocumentStore } from "@/store/documentStore";

const EJEMPLO_DBML = `Table users {
  id integer [pk, increment]
  email varchar(255) [not null, unique]
  creado timestamp [default: \`now()\`]
}

Table posts {
  id integer [pk]
  user_id integer [not null, ref: > users.id]
  titulo varchar(200)
}`;

const LANGUAGES: { value: SourceLanguage; label: string }[] = [
  { value: "dbml", label: "DBML (dbdiagram.io)" },
  { value: "mermaid", label: "Mermaid (flowchart)" },
];

const EJEMPLO_MERMAID = `flowchart LR
  usuario([Cliente])

  subgraph servicios[Servicios]
    api[API REST]
    cola{{Cola de eventos}}
  end

  base[(PostgreSQL)]

  usuario -- HTTPS --> api
  api --> base
  api -.-> cola`;

const EJEMPLOS: Record<SourceLanguage, string> = {
  dbml: EJEMPLO_DBML,
  mermaid: EJEMPLO_MERMAID,
};

export interface ImportDialogProps {
  onClose: () => void;
}

export function ImportDialog({ onClose }: ImportDialogProps) {
  const replaceDocument = useDocumentStore((s) => s.replaceDocument);

  const [language, setLanguage] = useState<SourceLanguage>("dbml");
  const [source, setSource] = useState("");
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  const errors = diagnostics.filter((d) => d.severity === "error");

  async function run() {
    if (!source.trim()) return;

    setBusy(true);
    setError(null);
    try {
      const result = await api.importCode(language, source, Date.now());
      setDiagnostics(result.diagnostics);

      // Con errores no se importa nada: un diagrama a medias es peor que
      // ninguno, porque parece completo.
      if (result.diagnostics.some((d) => d.severity === "error")) return;

      // El documento llega sin geometria; colocarlo es cosa de ELK.
      const document = result.document;
      await applyAutoLayout(document);

      replaceDocument(document);
      onClose();
    } catch (e) {
      setError(api.isAppError(e) ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="modal-backdrop" onPointerDown={onClose}>
      <div
        className="modal modal--wide"
        role="dialog"
        aria-modal="true"
        aria-label="Importar codigo"
        onPointerDown={(e) => e.stopPropagation()}
      >
        <h2 className="modal__title">Importar desde codigo</h2>

        <label className="modal__field">
          <span>Lenguaje</span>
          <select
            value={language}
            onChange={(e) => setLanguage(e.target.value as SourceLanguage)}
          >
            {LANGUAGES.map((l) => (
              <option key={l.value} value={l.value}>
                {l.label}
              </option>
            ))}
          </select>
        </label>

        <textarea
          className="modal__code selectable"
          value={source}
          onChange={(e) => {
            setSource(e.target.value);
            setDiagnostics([]);
          }}
          placeholder={EJEMPLOS[language]}
          spellCheck={false}
          rows={14}
        />

        {diagnostics.length > 0 && (
          <ul className="diagnostics">
            {diagnostics.map((d, index) => (
              <li key={index} className="diagnostics__item" data-severity={d.severity}>
                {d.span && (
                  <span className="diagnostics__where">
                    {d.span.line}:{d.span.column}
                  </span>
                )}
                <span>{d.message}</span>
              </li>
            ))}
          </ul>
        )}

        {error && <p className="modal__error">{error}</p>}

        <div className="modal__actions">
          <button
            type="button"
            onClick={() => setSource(EJEMPLOS[language])}
            disabled={busy}
          >
            Usar ejemplo
          </button>
          <span className="modal__spacer" />
          <button type="button" onClick={onClose} disabled={busy}>
            Cancelar
          </button>
          <button
            type="button"
            className="modal__primary"
            onClick={() => void run()}
            disabled={busy || !source.trim()}
          >
            {busy ? "Importando…" : errors.length > 0 ? "Reintentar" : "Importar"}
          </button>
        </div>
      </div>
    </div>
  );
}
