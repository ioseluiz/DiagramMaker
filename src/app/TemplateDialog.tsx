import { useEffect, useState } from "react";

import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { Template } from "@/model/generated/Template";
import * as api from "@/model/api";
import { openTemplate } from "./openTemplate";
import "./TemplateDialog.css";

const KIND_LABEL: Record<DiagramKind, string> = {
  er: "Entidad-relación",
  architecture: "Arquitectura",
  class: "Clases",
  useCase: "Casos de uso",
  sequence: "Secuencia",
  activity: "Actividad",
  state: "Estado",
};

export interface TemplateDialogProps {
  onClose: () => void;
}

/**
 * Diagramas de arranque.
 *
 * Una pantalla en blanco no dice que se puede hacer con el programa. Aqui hay
 * uno de cada tipo, hechos para abrirlos y cambiarlos.
 *
 * Llegan del backend como codigo y entran por el importador, igual que si se
 * hubieran pegado a mano: no hay un segundo camino que mantener, y lo que se
 * abre es un documento corriente desde el primer momento.
 */
export function TemplateDialog({ onClose }: TemplateDialogProps) {
  const [templates, setTemplates] = useState<Template[] | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  useEffect(() => {
    api
      .templateList()
      .then(setTemplates)
      .catch((e: unknown) => setError(api.isAppError(e) ? e.message : String(e)));
  }, []);

  async function abrir(template: Template) {
    setBusy(true);
    setError(null);
    try {
      await openTemplate(template);
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
        className="modal template-dialog"
        role="dialog"
        aria-modal="true"
        aria-label="Diagramas de arranque"
        onPointerDown={(e) => e.stopPropagation()}
      >
        <h2 className="modal__title">Empezar desde un ejemplo</h2>
        <p className="template-dialog__intro">
          Se abre como un diagrama normal: cámbialo a gusto.
        </p>

        {error && <p className="code-panel__error">{error}</p>}

        {templates === null && !error && <p>Cargando…</p>}

        {templates && (
          <ul className="template-dialog__list">
            {templates.map((template) => (
              <li key={template.id}>
                <button
                  type="button"
                  className="template-dialog__item"
                  disabled={busy}
                  onClick={() => void abrir(template)}
                >
                  <span className="template-dialog__kind">
                    {KIND_LABEL[template.kind]}
                  </span>
                  <span className="template-dialog__name">{template.name}</span>
                  <span className="template-dialog__hint">{template.description}</span>
                </button>
              </li>
            ))}
          </ul>
        )}

        <div className="modal__actions">
          <button type="button" onClick={onClose}>
            Cancelar
          </button>
        </div>
      </div>
    </div>
  );
}
