import type { Diagnostic } from "@/model/generated/Diagnostic";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { useDiagnostics } from "./useDiagnostics";
import "./DiagnosticsPanel.css";

/**
 * Lo que el documento tiene que decir, en un sitio donde se pueda actuar.
 *
 * La barra de estado ya contaba los problemas, pero un numero no sirve de
 * mucho: hay que poder leerlos y, sobre todo, llegar al nodo culpable. Cada
 * linea con `target` selecciona lo suyo al pulsarla.
 *
 * Se juntan dos origenes que hablan de cosas distintas:
 *
 * - **Del diagrama**: el informe de validacion, que se rehace solo con cada
 *   cambio y dice que le pasa al documento de ahora.
 * - **De la importacion**: lo que se perdio al leer el codigo. No cambia con
 *   lo que se dibuje despues, asi que se guarda aparte y dura hasta que
 *   llegue otro documento.
 */
export function DiagnosticsPanel() {
  const doc = useDocumentStore(selectDocument);
  const report = useDocumentStore((s) => s.report);
  const imported = useDocumentStore((s) => s.imported);
  const select = useDocumentStore((s) => s.select);
  const close = useDiagnostics((s) => s.close);

  const current = report?.diagnostics ?? [];

  return (
    <section className="diagnostics-panel" aria-label="Problemas del diagrama">
      <header className="diagnostics-panel__head">
        <h2 className="diagnostics-panel__title">Problemas</h2>
        <span className="diagnostics-panel__count">{resumen(current, imported)}</span>
        <button
          type="button"
          className="diagnostics-panel__close"
          onClick={close}
          aria-label="Cerrar el panel de problemas"
          title="Cerrar"
        >
          ×
        </button>
      </header>

      <div className="diagnostics-panel__body">
        {current.length === 0 && imported.length === 0 && (
          <p className="diagnostics-panel__empty">
            {doc ? "Nada que señalar." : "No hay ningún diagrama abierto."}
          </p>
        )}

        <Group
          title="Del diagrama"
          items={current}
          onPick={(target) => select([target])}
        />
        <Group title="De la importación" items={imported} />
      </div>
    </section>
  );
}

/** `3 problemas · 1 aviso`, o nada si no hay de que hablar. */
function resumen(current: Diagnostic[], imported: Diagnostic[]): string {
  const todos = [...current, ...imported];
  const errors = todos.filter((d) => d.severity === "error").length;
  const warnings = todos.filter((d) => d.severity === "warning").length;

  const partes: string[] = [];
  if (errors > 0) partes.push(`${errors} ${errors === 1 ? "problema" : "problemas"}`);
  if (warnings > 0) partes.push(`${warnings} ${warnings === 1 ? "aviso" : "avisos"}`);

  return partes.join(" · ");
}

interface GroupProps {
  title: string;
  items: Diagnostic[];
  onPick?: (target: string) => void;
}

function Group({ title, items, onPick }: GroupProps) {
  if (items.length === 0) return null;

  // Primero lo que impide algo y luego lo que solo avisa: leer de arriba
  // abajo tiene que ser leer de mas grave a menos.
  const sorted = [...items].sort((a, b) => peso(a) - peso(b));

  return (
    <>
      <h3 className="diagnostics-panel__group">{title}</h3>
      <ul className="diagnostics">
        {sorted.map((diagnostic, index) => (
          <Row
            key={`${diagnostic.code}-${index}`}
            diagnostic={diagnostic}
            onPick={onPick}
          />
        ))}
      </ul>
    </>
  );
}

function peso(diagnostic: Diagnostic): number {
  return { error: 0, warning: 1, info: 2 }[diagnostic.severity];
}

function Row({
  diagnostic,
  onPick,
}: {
  diagnostic: Diagnostic;
  onPick?: GroupProps["onPick"];
}) {
  const target = diagnostic.target;
  const pickable = Boolean(target && onPick);

  return (
    <li className="diagnostics__item" data-severity={diagnostic.severity}>
      {pickable ? (
        <button
          type="button"
          className="diagnostics__pick"
          onClick={() => onPick?.(target!)}
          title="Seleccionar lo que lo provoca"
        >
          {diagnostic.message}
        </button>
      ) : (
        <span className="diagnostics__text">{diagnostic.message}</span>
      )}

      {diagnostic.span && (
        <span className="diagnostics__where">
          línea {diagnostic.span.line}:{diagnostic.span.column}
        </span>
      )}
      <code className="diagnostics__code">{diagnostic.code}</code>
    </li>
  );
}
