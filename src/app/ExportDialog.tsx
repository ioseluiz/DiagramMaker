import { useEffect, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";

import type { ExportFormat } from "@/model/generated/ExportFormat";
import * as api from "@/model/api";
import { DEFAULT_EXPORT_OPTIONS, collectRenderInput } from "@/canvas/export/collect";
import { selectDocument, useDocumentStore } from "@/store/documentStore";

const FORMATS: { value: ExportFormat; label: string; hint: string }[] = [
  {
    value: "svg",
    label: "SVG",
    hint: "Vectorial y editable en Inkscape o Illustrator",
  },
  { value: "png", label: "PNG", hint: "Imagen de mapa de bits, con fondo opcional" },
  { value: "pdf", label: "PDF", hint: "Vectorial, con el texto seleccionable" },
];

const SCALES = [1, 2, 3, 4];

export interface ExportDialogProps {
  onClose: () => void;
}

export function ExportDialog({ onClose }: ExportDialogProps) {
  const doc = useDocumentStore(selectDocument);

  const [format, setFormat] = useState<ExportFormat>("svg");
  const [scale, setScale] = useState(DEFAULT_EXPORT_OPTIONS.scale);
  const [padding, setPadding] = useState(DEFAULT_EXPORT_OPTIONS.padding);
  const [opaque, setOpaque] = useState(DEFAULT_EXPORT_OPTIONS.opaqueBackground);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [onClose]);

  if (!doc) return null;

  const sanitized = doc.meta.title.replace(/[\\/:*?"<>|]/g, "").trim() || "diagrama";

  async function run() {
    if (!doc) return;

    setError(null);
    const path = await save({
      defaultPath: `${sanitized}.${format}`,
      filters: [{ name: format.toUpperCase(), extensions: [format] }],
    });
    if (!path) return;

    setBusy(true);
    try {
      const input = collectRenderInput(doc, {
        padding,
        opaqueBackground: opaque,
        scale,
      });
      await api.exportToFile(input, path, format);
      onClose();
    } catch (e) {
      setError(api.isAppError(e) ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function copySvg() {
    if (!doc) return;

    setError(null);
    setBusy(true);
    try {
      const svg = await api.exportSvg(
        collectRenderInput(doc, { padding, opaqueBackground: opaque, scale }),
      );
      await navigator.clipboard.writeText(svg);
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
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-label="Exportar diagrama"
        onPointerDown={(e) => e.stopPropagation()}
      >
        <h2 className="modal__title">Exportar</h2>

        <fieldset className="modal__formats">
          {FORMATS.map((option) => (
            <label key={option.value} className="modal__format">
              <input
                type="radio"
                name="formato"
                value={option.value}
                checked={format === option.value}
                onChange={() => setFormat(option.value)}
              />
              <span>
                <strong>{option.label}</strong>
                <small>{option.hint}</small>
              </span>
            </label>
          ))}
        </fieldset>

        <label className="modal__field">
          <span>Margen</span>
          <input
            type="number"
            min={0}
            max={200}
            step={4}
            value={padding}
            onChange={(e) => setPadding(Number(e.target.value))}
          />
        </label>

        {format === "png" && (
          <label className="modal__field">
            <span>Resolucion</span>
            <select value={scale} onChange={(e) => setScale(Number(e.target.value))}>
              {SCALES.map((s) => (
                <option key={s} value={s}>
                  {s}× {s === 2 ? "(recomendado)" : ""}
                </option>
              ))}
            </select>
          </label>
        )}

        <label className="modal__check">
          <input
            type="checkbox"
            checked={opaque}
            onChange={(e) => setOpaque(e.target.checked)}
          />
          Fondo opaco
          {!opaque && format === "pdf" && (
            <small> — un PDF sin fondo se vera sobre el blanco del visor</small>
          )}
        </label>

        {error && <p className="modal__error">{error}</p>}

        <div className="modal__actions">
          <button type="button" onClick={() => void copySvg()} disabled={busy}>
            Copiar SVG
          </button>
          <span className="modal__spacer" />
          <button type="button" onClick={onClose} disabled={busy}>
            Cancelar
          </button>
          <button
            type="button"
            className="modal__primary"
            onClick={() => void run()}
            disabled={busy}
          >
            {busy ? "Exportando…" : "Exportar"}
          </button>
        </div>
      </div>
    </div>
  );
}
