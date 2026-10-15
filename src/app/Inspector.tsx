import type { EdgeKind } from "@/model/generated/EdgeKind";
import type { NodeShape } from "@/model/generated/NodeShape";
import { SHAPE_LABEL } from "@/model/shapes";
import { erTableOf } from "@/model/table";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { TableInspector } from "./TableInspector";
import { setEdgeKind, setEndpointLabel, setLabel, setTitle } from "@/store/operations";

const EDGE_KIND_LABEL: Record<EdgeKind, string> = {
  association: "Asociacion",
  directed: "Dirigida",
  dependency: "Dependencia",
  inheritance: "Herencia",
  realization: "Realizacion",
  composition: "Composicion",
  aggregation: "Agregacion",
  flow: "Flujo",
};

/**
 * Panel de propiedades de lo seleccionado.
 *
 * Con cero o varios elementos seleccionados muestra las propiedades del
 * documento: es mas util que un panel vacio, y evita tener que ir a buscar el
 * titulo a otro sitio.
 */
export function Inspector() {
  const doc = useDocumentStore(selectDocument);
  const selection = useDocumentStore((s) => s.selection);
  const edit = useDocumentStore((s) => s.edit);

  if (!doc) return null;

  const only = selection.length === 1 ? selection[0] : undefined;
  const node = only ? doc.nodes.find((n) => n.id === only) : undefined;
  const edge = only ? doc.edges.find((e) => e.id === only) : undefined;

  const table = node ? erTableOf(node) : null;

  return (
    <aside className="inspector">
      {node && table && <TableInspector node={node} table={table} />}

      {node && !table && (
        <>
          <h2 className="inspector__title">Nodo</h2>
          <Field label="Etiqueta">
            <input
              value={node.label}
              onChange={(e) =>
                edit(
                  `label:${node.id}`,
                  (d) => setLabel(d, node.id, e.target.value),
                  true,
                )
              }
            />
          </Field>
          <Field label="Forma">
            <span className="inspector__readonly">
              {SHAPE_LABEL[node.shape as NodeShape]}
            </span>
          </Field>
          {node.geometry && (
            <Field label="Posicion">
              <span className="inspector__readonly">
                {Math.round(node.geometry.x)}, {Math.round(node.geometry.y)} ·{" "}
                {Math.round(node.geometry.width)}×{Math.round(node.geometry.height)}
              </span>
            </Field>
          )}
        </>
      )}

      {edge && (
        <>
          <h2 className="inspector__title">Relacion</h2>
          <Field label="Etiqueta">
            <input
              value={edge.label ?? ""}
              onChange={(e) =>
                edit(
                  `label:${edge.id}`,
                  (d) => setLabel(d, edge.id, e.target.value),
                  true,
                )
              }
            />
          </Field>
          <Field label="Tipo">
            <select
              value={edge.kind}
              onChange={(e) =>
                edit("tipo de relacion", (d) =>
                  setEdgeKind(d, edge.id, e.target.value as EdgeKind),
                )
              }
            >
              {Object.entries(EDGE_KIND_LABEL).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Cardinalidad origen">
            <input
              value={edge.source.label ?? ""}
              placeholder="1"
              onChange={(e) =>
                edit(
                  `card:${edge.id}:source`,
                  (d) => setEndpointLabel(d, edge.id, "source", e.target.value),
                  true,
                )
              }
            />
          </Field>
          <Field label="Cardinalidad destino">
            <input
              value={edge.target.label ?? ""}
              placeholder="0..*"
              onChange={(e) =>
                edit(
                  `card:${edge.id}:target`,
                  (d) => setEndpointLabel(d, edge.id, "target", e.target.value),
                  true,
                )
              }
            />
          </Field>
        </>
      )}

      {!node && !edge && (
        <>
          <h2 className="inspector__title">Documento</h2>
          <Field label="Titulo">
            <input
              value={doc.meta.title}
              onChange={(e) => edit("titulo", (d) => setTitle(d, e.target.value), true)}
            />
          </Field>
          <Field label="Contenido">
            <span className="inspector__readonly">
              {doc.nodes.length} nodos · {doc.edges.length} relaciones
            </span>
          </Field>
          {selection.length > 1 && (
            <p className="inspector__hint">
              {selection.length} elementos seleccionados. Selecciona uno solo para
              editar sus propiedades.
            </p>
          )}
        </>
      )}
    </aside>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="inspector__field">
      <span className="inspector__label">{label}</span>
      {children}
    </label>
  );
}
