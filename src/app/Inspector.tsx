import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { EdgeKind } from "@/model/generated/EdgeKind";
import type { Node } from "@/model/generated/Node";
import type { NodeShape } from "@/model/generated/NodeShape";
import { C4_LEVELS, c4LevelById, c4LevelOf } from "@/model/c4";
import { SHAPE_LABEL } from "@/model/shapes";
import { boxForLabel, fontSizeFor } from "@/canvas/text";
import { nodeStyle } from "@/model/style";
import { classBoxOf } from "@/model/classBox";
import { erTableOf } from "@/model/table";
import { ClassInspector } from "./ClassInspector";
import { ColorPicker } from "./ColorPicker";
import { SequenceInspector } from "./SequenceInspector";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { TableInspector } from "./TableInspector";
import {
  RELATION_LABELS,
  nextFontSize,
  relationKindOf,
  setEdgeKind,
  setEndpointLabel,
  setLabel,
  setNodeC4Level,
  setNodeFill,
  setNodeFontSize,
  setRelationKind,
  setTitle,
  type RelationKind,
} from "@/store/operations";

const RELATION_LABEL: Record<RelationKind, string> = {
  manyToOne: "Muchos a uno  (* → 1)",
  oneToMany: "Uno a muchos  (1 → *)",
  oneToOne: "Uno a uno  (1 → 1)",
  manyToMany: "Muchos a muchos  (* → *)",
};

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
  const classBox = node ? classBoxOf(node) : null;

  // Una secuencia se edita por su guion, no por propiedades de nodos y
  // aristas: el panel entero es otro.
  if (doc.kind === "sequence") {
    return (
      <aside className="inspector">
        <SequenceInspector document={doc} />
      </aside>
    );
  }

  return (
    <aside className="inspector">
      {node && table && <TableInspector node={node} table={table} />}

      {node && classBox && <ClassInspector node={node} box={classBox} />}

      {node && !table && !classBox && (
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
          {/*
            C4 no es otro tipo de diagrama, es una convencion de colores
            sobre uno de arquitectura: el nivel se ofrece ahi y en ningun
            otro sitio. Elegir uno pisa el color, y elegir un color quita el
            nivel; son dos maneras de escribir lo mismo.
          */}
          {doc.kind === "architecture" && (
            <Field label="Nivel C4">
              <select
                value={c4LevelOf(nodeStyle(node).fill)?.id ?? ""}
                onChange={(e) =>
                  edit("nivel C4", (d) =>
                    setNodeC4Level(d, node.id, c4LevelById(e.target.value) ?? null),
                  )
                }
              >
                <option value="">Ninguno</option>
                {C4_LEVELS.map((level) => (
                  <option key={level.id} value={level.id} title={level.hint}>
                    {level.name}
                  </option>
                ))}
              </select>
            </Field>
          )}
          <div className="inspector__field">
            <span className="inspector__label">Color</span>
            <ColorPicker
              value={nodeStyle(node).fill ?? null}
              onChange={(fill) => edit("color", (d) => setNodeFill(d, node.id, fill))}
            />
          </div>
          <div className="inspector__field">
            <span className="inspector__label">Tamano del texto</span>
            <FontSizeStepper
              node={node}
              onChange={(size) =>
                edit("tamano del texto", (d) =>
                  setNodeFontSize(
                    d,
                    node.id,
                    size,
                    // La caja se mide aqui, en el navegador, y se le pasa
                    // hecha: el documento no sabe medir texto.
                    boxForLabel(node.label, size ?? fontSizeFor(node.shape), {
                      bold: nodeStyle(node).bold,
                      italic: nodeStyle(node).italic,
                    }),
                  ),
                )
              }
            />
          </div>
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
          {/*
            En un entidad-relacion la cardinalidad son cuatro casos, no texto
            libre: dejarla abierta invita a escribir `N` o `muchos` y que
            luego el generador de DBML no sepa que hacer con ello. En el
            resto de diagramas la multiplicidad si es libre.
          */}
          {doc.kind === "er" ? (
            <>
              <Field label="Cardinalidad">
                <select
                  value={relationKindOf(edge)}
                  onChange={(e) =>
                    edit("cardinalidad", (d) =>
                      setRelationKind(d, edge.id, e.target.value as RelationKind),
                    )
                  }
                >
                  {(Object.keys(RELATION_LABELS) as RelationKind[]).map((kind) => (
                    <option key={kind} value={kind}>
                      {RELATION_LABEL[kind]}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="Columnas">
                <span className="inspector__readonly">{columnsOf(doc, edge)}</span>
              </Field>
            </>
          ) : (
            <>
              <Field label="Multiplicidad origen">
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
              <Field label="Multiplicidad destino">
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

/**
 * Que columnas une la relacion, en texto. Si algun extremo no esta anclado a
 * una columna concreta se dice, porque es justo lo que hay que arreglar para
 * que el DBML generado tenga sentido.
 */
function columnsOf(doc: Document, edge: Edge): string {
  const side = (end: Edge["source"]) => {
    const node = doc.nodes.find((n) => n.id === end.node);
    if (!node) return "?";
    const table = erTableOf(node);
    const column = end.port ? table?.columns.find((c) => c.id === end.port) : undefined;
    return column ? `${node.label}.${column.name}` : `${node.label} (tabla)`;
  };

  return `${side(edge.source)} → ${side(edge.target)}`;
}

/**
 * Menos, el cuerpo actual, mas.
 *
 * Muestra siempre el tamano efectivo —el heredado de la forma cuando el nodo
 * no tiene uno propio— para que el numero de la pantalla sea el de la letra
 * que se ve, y no un hueco en blanco.
 */
function FontSizeStepper({
  node,
  onChange,
}: {
  node: Node;
  onChange: (size: number | null) => void;
}) {
  const propio = nodeStyle(node).fontSize;
  const actual = propio ?? fontSizeFor(node.shape);

  const menor = nextFontSize(actual, -1);
  const mayor = nextFontSize(actual, 1);

  return (
    <div className="inspector__stepper">
      <button
        type="button"
        onClick={() => onChange(menor)}
        disabled={menor === actual}
        title="Reducir el texto"
      >
        −
      </button>
      <span className="inspector__stepper-value">{actual}</span>
      <button
        type="button"
        onClick={() => onChange(mayor)}
        disabled={mayor === actual}
        title="Aumentar el texto"
      >
        +
      </button>
      <button
        type="button"
        className="inspector__stepper-reset"
        onClick={() => onChange(null)}
        disabled={propio === undefined}
        title="Volver al tamano de la forma"
      >
        Auto
      </button>
    </div>
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
