import { useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";

import type { SourceLanguage } from "@/model/generated/SourceLanguage";
import * as api from "@/model/api";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { useCodePanel } from "./useCodePanel";
import "./CodePanel.css";

/**
 * Codigo del diagrama, siempre al dia.
 *
 * Es de solo lectura a proposito. El camino de vuelta —escribir codigo y que
 * el diagrama cambie— existe, pero por el dialogo de importar: reescribir el
 * codigo a cada tecla y volver a montar el diagrama perderia las posiciones
 * que el usuario ha ido ajustando.
 */

/** Que lenguaje corresponde a cada tipo de diagrama. */
const LANGUAGE_BY_KIND: Partial<Record<string, SourceLanguage>> = {
  er: "dbml",
};

const EXTENSION: Record<SourceLanguage, string> = { dbml: "dbml" };
const LANGUAGE_NAME: Record<SourceLanguage, string> = { dbml: "DBML" };

/**
 * Espera antes de regenerar. Arrastrar un nodo produce decenas de cambios por
 * segundo y cada uno cruzaria el puente IPC.
 */
const DEBOUNCE_MS = 250;

export function CodePanel() {
  const doc = useDocumentStore(selectDocument);
  const close = useCodePanel((s) => s.close);

  const [code, setCode] = useState("");
  const [copied, setCopied] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const language = doc ? LANGUAGE_BY_KIND[doc.kind] : undefined;

  useEffect(() => {
    // Sin generador para este tipo de diagrama no hay nada que regenerar; el
    // panel muestra su aviso y `code` no llega a pintarse.
    if (!doc || !language) return;

    const timer = setTimeout(() => {
      api
        .emitCode(language, doc)
        .then((text) => {
          setCode(text);
          setError(null);
        })
        .catch((e: unknown) => setError(api.isAppError(e) ? e.message : String(e)));
    }, DEBOUNCE_MS);

    return () => clearTimeout(timer);
  }, [doc, language]);

  // El aviso de copiado vuelve a su sitio solo; si no, el boton se quedaria
  // diciendo "Copiado" para siempre.
  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1_500);
    return () => clearTimeout(timer);
  }, [copied]);

  async function copy() {
    try {
      await navigator.clipboard.writeText(code);
      setCopied(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveToFile() {
    if (!doc || !language) return;

    const name = doc.meta.title.replace(/[\\/:*?"<>|]/g, "").trim() || "esquema";
    const path = await save({
      defaultPath: `${name}.${EXTENSION[language]}`,
      filters: [{ name: LANGUAGE_NAME[language], extensions: [EXTENSION[language]] }],
    });
    if (!path) return;

    try {
      await api.emitCodeToFile(language, doc, path);
    } catch (e) {
      setError(api.isAppError(e) ? e.message : String(e));
    }
  }

  return (
    <aside className="code-panel">
      <header className="code-panel__head">
        <span className="code-panel__title">
          {language ? LANGUAGE_NAME[language] : "Código"}
        </span>
        <button
          type="button"
          className="code-panel__close"
          onClick={close}
          aria-label="Cerrar el panel de código"
          title="Cerrar"
        >
          ×
        </button>
      </header>

      {language ? (
        <>
          <Code text={code} />

          {error && <p className="code-panel__error">{error}</p>}

          <div className="code-panel__actions">
            <button type="button" onClick={() => void copy()} disabled={!code}>
              {copied ? "Copiado" : "Copiar"}
            </button>
            <button type="button" onClick={() => void saveToFile()} disabled={!code}>
              Guardar como…
            </button>
          </div>
        </>
      ) : (
        <p className="code-panel__empty">
          Todavía no hay generador de código para este tipo de diagrama. El de
          entidad-relación produce DBML.
        </p>
      )}
    </aside>
  );
}

/**
 * Muestra el codigo conservando la posicion del scroll entre regeneraciones.
 *
 * Sin esto, mover un nodo devolveria el panel al principio cada cuarto de
 * segundo y seria imposible leer un esquema largo mientras se trabaja.
 */
function Code({ text }: { text: string }) {
  const ref = useRef<HTMLPreElement>(null);
  const scroll = useRef(0);

  useEffect(() => {
    if (ref.current) ref.current.scrollTop = scroll.current;
  }, [text]);

  return (
    <pre
      ref={ref}
      className="code-panel__code selectable"
      onScroll={(e) => {
        scroll.current = e.currentTarget.scrollTop;
      }}
    >
      {text}
    </pre>
  );
}
