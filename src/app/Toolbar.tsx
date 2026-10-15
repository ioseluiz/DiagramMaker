import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { NodeShape } from "@/model/generated/NodeShape";
import { SHAPE_LABEL } from "@/model/shapes";
import {
  selectCanRedo,
  selectCanUndo,
  selectDocument,
  useDocumentStore,
} from "@/store/documentStore";
import { addNode, addTable, setDiagramKind } from "@/store/operations";
import { useTheme, type ThemePreference } from "@/theme/useTheme";
import { useState } from "react";

import { ExportDialog } from "./ExportDialog";
import { ImportDialog } from "./ImportDialog";
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

/**
 * Formas que ofrece la paleta, segun el tipo de diagrama.
 *
 * En un entidad-relacion lo que se dibuja son tablas; ofrecer un rombo o un
 * actor seria ruido. El resto de tipos comparten el juego general hasta que
 * cada uno tenga el suyo.
 */
const PALETTE_BY_KIND: Partial<Record<DiagramKind, NodeShape[]>> = {
  er: ["table", "package", "note"],
};

const DEFAULT_PALETTE: NodeShape[] = [
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
  const [importing, setImporting] = useState(false);

  const palette = (doc && PALETTE_BY_KIND[doc.kind]) ?? DEFAULT_PALETTE;

  const addShape = (shape: NodeShape) => {
    const position = nextPosition(doc?.nodes.length ?? 0);

    edit("anadir", (draft) => {
      if (shape === "table") {
        addTable(draft, position);
        return;
      }
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
          onClick={() => setImporting(true)}
          title="Importar desde DBML"
        >
          Importar
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
        {palette.map((shape) => (
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

      <label className="toolbar__field">
        Diagrama
        <select
          value={doc?.kind ?? "architecture"}
          disabled={!doc}
          onChange={(e) =>
            edit("tipo de diagrama", (d) =>
              setDiagramKind(d, e.target.value as DiagramKind),
            )
          }
        >
          {(Object.keys(DIAGRAM_LABEL) as DiagramKind[]).map((kind) => (
            <option key={kind} value={kind}>
              {DIAGRAM_LABEL[kind]}
            </option>
          ))}
        </select>
      </label>

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
      {importing && <ImportDialog onClose={() => setImporting(false)} />}
    </header>
  );
}

/**
 * Miniatura de cada forma para el boton de la paleta.
 *
 * Todas se dibujan en una rejilla cuadrada de 24x24 y ocupan el mismo
 * recuadro interior (de 3 a 21 en ambos ejes), de modo que el conjunto se ve
 * alineado aunque las formas sean muy distintas entre si. Antes el `viewBox`
 * era 24x20 y se pintaba a 20x17: dos relaciones de aspecto distintas.
 */
function ShapeIcon({ shape }: { shape: NodeShape }) {
  const common = {
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.5,
    strokeLinejoin: "round" as const,
    strokeLinecap: "round" as const,
  };

  return (
    <svg
      viewBox="0 0 24 24"
      width="18"
      height="18"
      aria-hidden
      focusable="false"
      className="toolbar__icon"
    >
      {shape === "rectangle" && <rect x="3" y="6" width="18" height="12" {...common} />}
      {shape === "roundedRect" && (
        <rect x="3" y="6" width="18" height="12" rx="4" {...common} />
      )}
      {shape === "ellipse" && <ellipse cx="12" cy="12" rx="9" ry="6" {...common} />}
      {shape === "diamond" && <path d="M12 4 L21 12 L12 20 L3 12 Z" {...common} />}
      {shape === "cylinder" && (
        <>
          <path
            d="M3 7.5 A9 2.5 0 0 1 21 7.5 L21 16.5 A9 2.5 0 0 1 3 16.5 Z"
            {...common}
          />
          <path d="M3 7.5 A9 2.5 0 0 0 21 7.5" {...common} />
        </>
      )}
      {shape === "note" && <path d="M4 5 L17 5 L20 8 L20 19 L4 19 Z" {...common} />}
      {shape === "package" && (
        <>
          <path d="M3 8 L3 5 L10 5 L10 8" {...common} />
          <rect x="3" y="8" width="18" height="11" {...common} />
        </>
      )}
      {shape === "table" && (
        <>
          <rect x="3" y="5" width="18" height="14" rx="2" {...common} />
          <path d="M3 9.5 L21 9.5 M3 14.25 L21 14.25" {...common} />
        </>
      )}
      {shape === "actor" && (
        <>
          <circle cx="12" cy="6.5" r="2.5" {...common} />
          <path
            d="M12 9 L12 15 M7 11.5 L17 11.5 M12 15 L8.5 20 M12 15 L15.5 20"
            {...common}
          />
        </>
      )}
    </svg>
  );
}
