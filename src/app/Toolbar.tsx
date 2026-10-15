import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { NodeShape } from "@/model/generated/NodeShape";
import { SHAPE_LABEL } from "@/model/shapes";
import {
  selectCanRedo,
  selectCanUndo,
  selectDocument,
  useDocumentStore,
} from "@/store/documentStore";
import { addNode } from "@/store/operations";
import { useTheme, type ThemePreference } from "@/theme/useTheme";
import { useState } from "react";

import { ExportDialog } from "./ExportDialog";
import { RecentMenu } from "./RecentMenu";
import { useDocumentCommands } from "./useDocumentCommands";

const DIAGRAM_LABEL: Record<DiagramKind, string> = {
  er: "Entidad-relacion",
  architecture: "Arquitectura",
  class: "Clases",
  useCase: "Casos de uso",
  sequence: "Secuencia",
  activity: "Actividad",
  state: "Estado",
};

const THEME_LABEL: Record<ThemePreference, string> = {
  system: "Sistema",
  light: "Claro",
  dark: "Oscuro",
};

const PALETTE: NodeShape[] = [
  "rectangle",
  "roundedRect",
  "ellipse",
  "diamond",
  "cylinder",
  "note",
  "package",
  "actor",
];

/**
 * Coloca los nodos nuevos en cascada para que no se apilen uno encima de otro.
 * No intenta adivinar el centro del viewport: una cascada predecible molesta
 * menos que un nodo que aparece donde el usuario no mira.
 */
function nextPosition(count: number): { x: number; y: number } {
  const column = count % 4;
  const row = Math.floor(count / 4);
  return { x: 80 + column * 200, y: 80 + row * 120 };
}

export function Toolbar() {
  const doc = useDocumentStore(selectDocument);
  const dirty = useDocumentStore((s) => s.dirty);
  const busy = useDocumentStore((s) => s.busy);
  const canUndo = useDocumentStore(selectCanUndo);
  const canRedo = useDocumentStore(selectCanRedo);
  const edit = useDocumentStore((s) => s.edit);
  const undo = useDocumentStore((s) => s.undo);
  const redo = useDocumentStore((s) => s.redo);

  const { createNew, openFile, saveFile } = useDocumentCommands();
  const { preference, setPreference } = useTheme();
  const [exporting, setExporting] = useState(false);

  const addShape = (shape: NodeShape) => {
    const position = nextPosition(doc?.nodes.length ?? 0);
    edit("anadir", (draft) => {
      addNode(draft, { shape, label: SHAPE_LABEL[shape], ...position });
    });
  };

  return (
    <header className="toolbar">
      <div className="toolbar__group">
        <button type="button" onClick={() => createNew()} title="Nuevo (Ctrl+N)">
          Nuevo
        </button>
        <button type="button" onClick={() => void openFile()} title="Abrir (Ctrl+O)">
          Abrir
        </button>
        <RecentMenu />
        <button
          type="button"
          onClick={() => void saveFile()}
          disabled={!doc || busy}
          title="Guardar (Ctrl+S)"
        >
          Guardar{dirty ? " •" : ""}
        </button>
        <button
          type="button"
          onClick={() => setExporting(true)}
          disabled={!doc}
          title="Exportar a SVG, PNG o PDF"
        >
          Exportar
        </button>
      </div>

      <div className="toolbar__group">
        <button
          type="button"
          onClick={undo}
          disabled={!canUndo}
          title="Deshacer (Ctrl+Z)"
        >
          Deshacer
        </button>
        <button
          type="button"
          onClick={redo}
          disabled={!canRedo}
          title="Rehacer (Ctrl+Shift+Z)"
        >
          Rehacer
        </button>
      </div>

      <div className="toolbar__group toolbar__palette">
        {PALETTE.map((shape) => (
          <button
            key={shape}
            type="button"
            className="toolbar__shape"
            onClick={() => addShape(shape)}
            disabled={!doc}
            title={`Anadir ${SHAPE_LABEL[shape].toLowerCase()}`}
          >
            <ShapeIcon shape={shape} />
          </button>
        ))}
      </div>

      <span className="toolbar__spacer" />

      <span className="toolbar__kind">{doc ? DIAGRAM_LABEL[doc.kind] : ""}</span>

      <label className="toolbar__field">
        Tema
        <select
          value={preference}
          onChange={(e) => setPreference(e.target.value as ThemePreference)}
        >
          {(["system", "light", "dark"] as const).map((p) => (
            <option key={p} value={p}>
              {THEME_LABEL[p]}
            </option>
          ))}
        </select>
      </label>

      {exporting && <ExportDialog onClose={() => setExporting(false)} />}
    </header>
  );
}

/** Miniatura de cada forma para el boton de la paleta. */
function ShapeIcon({ shape }: { shape: NodeShape }) {
  const common = {
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.5,
  };

  return (
    <svg viewBox="0 0 24 20" width="20" height="17" aria-hidden>
      {shape === "rectangle" && <rect x="2" y="4" width="20" height="12" {...common} />}
      {shape === "roundedRect" && (
        <rect x="2" y="4" width="20" height="12" rx="4" {...common} />
      )}
      {shape === "ellipse" && <ellipse cx="12" cy="10" rx="10" ry="6" {...common} />}
      {shape === "diamond" && <path d="M12 3 L22 10 L12 17 L2 10 Z" {...common} />}
      {shape === "cylinder" && (
        <>
          <path d="M3 6 A9 3 0 0 1 21 6 L21 14 A9 3 0 0 1 3 14 Z" {...common} />
          <path d="M3 6 A9 3 0 0 0 21 6" {...common} />
        </>
      )}
      {shape === "note" && <path d="M3 3 L17 3 L21 7 L21 17 L3 17 Z" {...common} />}
      {shape === "package" && (
        <>
          <path d="M2 6 L2 3 L10 3 L10 6" {...common} />
          <rect x="2" y="6" width="20" height="11" {...common} />
        </>
      )}
      {shape === "actor" && (
        <>
          <circle cx="12" cy="5" r="2.5" {...common} />
          <path
            d="M12 7.5 L12 13 M7 9.5 L17 9.5 M12 13 L8 18 M12 13 L16 18"
            {...common}
          />
        </>
      )}
    </svg>
  );
}
