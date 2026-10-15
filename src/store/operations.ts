import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { EdgeKind } from "@/model/generated/EdgeKind";
import type { Node } from "@/model/generated/Node";
import type { NodeShape } from "@/model/generated/NodeShape";
import type { ErColumn } from "@/model/generated/ErColumn";
import type { ErTable } from "@/model/generated/ErTable";
import type { Side } from "@/model/generated/Side";
import { defaultSizeFor } from "@/model/shapes";
import { tableSize } from "@/model/table";

/**
 * Operaciones sobre el documento.
 *
 * Todas mutan un borrador de immer y no devuelven estado: el historial se
 * encarga de convertirlas en parches. Son funciones puras respecto al
 * borrador, asi que se prueban sin tocar React ni el store.
 */

export function newId(prefix: string): string {
  const rand =
    typeof crypto !== "undefined" && "randomUUID" in crypto
      ? crypto.randomUUID().slice(0, 8)
      : Math.random().toString(36).slice(2, 10);

  return `${prefix}_${rand}`;
}

export interface AddNodeOptions {
  shape: NodeShape;
  label?: string;
  x: number;
  y: number;
  parent?: string;
}

export function addNode(draft: Document, options: AddNodeOptions): string {
  const id = newId("n");
  const [width, height] = defaultSizeFor(options.shape);

  const node: Node = {
    id,
    label: options.label ?? "",
    shape: options.shape,
    geometry: { x: options.x, y: options.y, width, height },
    // Un nodo creado a mano nace anclado: el auto-layout no debe moverlo.
    pinned: true,
    style: {},
    data: { type: "plain" },
  };

  if (options.parent) node.parent = options.parent;

  draft.nodes.push(node);
  return id;
}

export interface ConnectSides {
  source?: Side;
  target?: Side;
}

/** Columnas concretas a las que se engancha cada extremo, si las hay. */
export interface ConnectPorts {
  source?: string;
  target?: string;
}

/**
 * Cardinalidad de una relacion, como par de etiquetas de extremo.
 *
 * Son las cuatro que entiende DBML, con los mismos simbolos que usa el
 * importador para no tener dos vocabularios.
 */
export type RelationKind = "manyToOne" | "oneToMany" | "oneToOne" | "manyToMany";

export const RELATION_LABELS: Record<RelationKind, [string, string]> = {
  manyToOne: ["*", "1"],
  oneToMany: ["1", "*"],
  oneToOne: ["1", "1"],
  manyToMany: ["*", "*"],
};

/** Deduce la cardinalidad a partir de las etiquetas guardadas. */
export function relationKindOf(edge: Edge): RelationKind {
  const from = edge.source.label === "*";
  const to = edge.target.label === "*";

  if (from && to) return "manyToMany";
  if (from) return "manyToOne";
  if (to) return "oneToMany";
  return "oneToOne";
}

/**
 * Crea la relacion. `sides` es el borde desde el que arrastro el usuario: se
 * guarda y manda sobre cualquier calculo automatico. Omitirlo deja que el
 * lienzo decida segun la posicion de las cajas, que es el caso de los
 * diagramas importados.
 */
export function connect(
  draft: Document,
  source: string,
  target: string,
  kind: EdgeKind = "association",
  sides: ConnectSides = {},
  ports: ConnectPorts = {},
): string | null {
  const exists = (id: string) => draft.nodes.some((n) => n.id === id);
  if (!exists(source) || !exists(target)) return null;

  const id = newId("e");
  const edge: Edge = {
    id,
    source: { node: source, side: sides.source, port: ports.source },
    target: { node: target, side: sides.target, port: ports.target },
    kind,
    routing: "orthogonal",
    style: {},
  };

  draft.edges.push(edge);

  // Unir dos columnas en un entidad-relacion es declarar una clave ajena. El
  // caso corriente con diferencia es muchos-a-uno: la columna desde la que se
  // arrastra apunta a la clave de la otra tabla. Se deja puesto y editable en
  // vez de obligar a rellenarlo cada vez.
  if (draft.kind === "er" && ports.source && ports.target) {
    setRelationKind(draft, id, "manyToOne");
  }

  return id;
}

/**
 * Fija la cardinalidad y recalcula que columna es la clave ajena.
 *
 * La clave ajena vive en el lado "muchos": es la tabla que repite valores de
 * la otra. En uno-a-uno y muchos-a-muchos no hay un lado evidente, asi que no
 * se marca nada.
 */
export function setRelationKind(
  draft: Document,
  edgeId: string,
  kind: RelationKind,
): void {
  const edge = draft.edges.find((e) => e.id === edgeId);
  if (!edge) return;

  const [from, to] = RELATION_LABELS[kind];
  edge.source.label = from;
  edge.target.label = to;

  const many =
    from === "*" && to === "1"
      ? edge.source
      : to === "*" && from === "1"
        ? edge.target
        : null;

  for (const end of [edge.source, edge.target]) {
    if (!end.port) continue;
    const table = tableOf(draft, end.node);
    const column = table?.columns.find((c) => c.id === end.port);
    if (column) column.foreignKey = end === many;
  }
}

/**
 * Reengancha un extremo a otro nodo o a otro lado. Es lo que ocurre al
 * arrastrar la punta de una arista ya existente.
 */
export function reconnect(
  draft: Document,
  edgeId: string,
  end: "source" | "target",
  node: string,
  side?: Side,
  port?: string,
): void {
  const edge = draft.edges.find((e) => e.id === edgeId);
  if (!edge) return;
  if (!draft.nodes.some((n) => n.id === node)) return;

  edge[end].node = node;
  edge[end].side = side;
  edge[end].port = port;
}

/** Fija o libera el lado de un extremo. `undefined` vuelve a automatico. */
export function setEndpointSide(
  draft: Document,
  edgeId: string,
  end: "source" | "target",
  side: Side | undefined,
): void {
  const edge = draft.edges.find((e) => e.id === edgeId);
  if (edge) edge[end].side = side;
}

export function moveNode(draft: Document, id: string, x: number, y: number): void {
  const node = draft.nodes.find((n) => n.id === id);
  if (!node?.geometry) return;

  node.geometry.x = x;
  node.geometry.y = y;
  node.pinned = true;
}

export function resizeNode(
  draft: Document,
  id: string,
  width: number,
  height: number,
): void {
  const node = draft.nodes.find((n) => n.id === id);
  if (!node?.geometry) return;

  node.geometry.width = width;
  node.geometry.height = height;
}

export function setLabel(draft: Document, id: string, label: string): void {
  const node = draft.nodes.find((n) => n.id === id);
  if (node) {
    node.label = label;
    return;
  }

  const edge = draft.edges.find((e) => e.id === id);
  if (edge) edge.label = label || undefined;
}

/**
 * Borra nodos y aristas por id.
 *
 * Borrar un nodo arrastra lo que cuelga de el: sus aristas y, recursivamente,
 * los nodos que contiene. Dejar aristas colgando produciria un documento que
 * `validate()` rechaza.
 */
export function removeElements(draft: Document, ids: readonly string[]): void {
  const doomed = new Set(ids);

  let grew = true;
  while (grew) {
    grew = false;
    for (const node of draft.nodes) {
      if (node.parent && doomed.has(node.parent) && !doomed.has(node.id)) {
        doomed.add(node.id);
        grew = true;
      }
    }
  }

  draft.nodes = draft.nodes.filter((n) => !doomed.has(n.id));
  draft.edges = draft.edges.filter(
    (e) =>
      !doomed.has(e.id) && !doomed.has(e.source.node) && !doomed.has(e.target.node),
  );
}

export function setEndpointLabel(
  draft: Document,
  edgeId: string,
  end: "source" | "target",
  label: string,
): void {
  const edge = draft.edges.find((e) => e.id === edgeId);
  if (edge) edge[end].label = label || undefined;
}

export function setEdgeKind(draft: Document, edgeId: string, kind: EdgeKind): void {
  const edge = draft.edges.find((e) => e.id === edgeId);
  if (edge) edge.kind = kind;
}

export function setTitle(draft: Document, title: string): void {
  draft.meta.title = title;
}

/**
 * Cambia el tipo de diagrama.
 *
 * No toca los nodos: cada uno se sigue dibujando segun su forma. Lo que
 * cambia es que formas ofrece la paleta y a que lenguaje se exporta el
 * codigo.
 */
export function setDiagramKind(draft: Document, kind: DiagramKind): void {
  draft.kind = kind;
}

/* --- Tablas entidad-relacion -------------------------------------------- */

/** Tabla de un nodo, si lo es. */
function tableOf(draft: Document, nodeId: string): ErTable | null {
  const node = draft.nodes.find((n) => n.id === nodeId);
  return node?.data.type === "er" ? node.data : null;
}

/**
 * Reajusta la caja tras cambiar las columnas.
 *
 * El alto es derivado: una fila por columna, siempre. El ancho solo crece —
 * si el usuario ensancho la tabla a mano, encogersela porque borro una
 * columna larga seria pelearse con el.
 */
function resyncTableGeometry(draft: Document, nodeId: string): void {
  const node = draft.nodes.find((n) => n.id === nodeId);
  if (!node || node.data.type !== "er") return;

  const natural = tableSize(node);
  node.geometry = {
    x: node.geometry?.x ?? 0,
    y: node.geometry?.y ?? 0,
    width: Math.max(node.geometry?.width ?? 0, natural.width),
    height: natural.height,
  };
}

export interface AddTableOptions {
  x: number;
  y: number;
  label?: string;
}

/**
 * Crea una tabla con una clave primaria `id`, que es con lo que empieza
 * practicamente cualquier tabla.
 */
export function addTable(draft: Document, options: AddTableOptions): string {
  const id = newId("t");

  const node: Node = {
    id,
    label: options.label ?? "tabla",
    shape: "table",
    pinned: true,
    style: {},
    data: {
      type: "er",
      columns: [
        {
          id: newId("c"),
          name: "id",
          dataType: "integer",
          primaryKey: true,
          foreignKey: false,
          nullable: false,
          unique: false,
          increment: true,
        },
      ],
    },
  };

  draft.nodes.push(node);
  node.geometry = { x: options.x, y: options.y, ...tableSize(node) };

  return id;
}

/** Nombre libre para una columna nueva: `columna`, `columna_2`, ... */
function freeColumnName(table: ErTable): string {
  const taken = new Set(table.columns.map((c) => c.name.toLowerCase()));
  if (!taken.has("columna")) return "columna";

  for (let i = 2; ; i++) {
    const candidate = `columna_${i}`;
    if (!taken.has(candidate)) return candidate;
  }
}

export function addColumn(
  draft: Document,
  nodeId: string,
  name?: string,
): string | null {
  const table = tableOf(draft, nodeId);
  if (!table) return null;

  const id = newId("c");
  table.columns.push({
    id,
    name: name ?? freeColumnName(table),
    dataType: "",
    primaryKey: false,
    foreignKey: false,
    nullable: true,
    unique: false,
    increment: false,
  });

  resyncTableGeometry(draft, nodeId);
  return id;
}

/**
 * Borra una columna.
 *
 * Las relaciones que apuntaban a ella **no se borran**: pierden el anclaje a
 * la columna concreta y quedan apuntando a la tabla entera. Borrar la
 * relacion del usuario porque toco una columna seria pasarse.
 */
export function removeColumn(draft: Document, nodeId: string, columnId: string): void {
  const table = tableOf(draft, nodeId);
  if (!table) return;

  table.columns = table.columns.filter((c) => c.id !== columnId);

  for (const edge of draft.edges) {
    if (edge.source.port === columnId) edge.source.port = undefined;
    if (edge.target.port === columnId) edge.target.port = undefined;
  }

  resyncTableGeometry(draft, nodeId);
}

export function updateColumn(
  draft: Document,
  nodeId: string,
  columnId: string,
  changes: Partial<Omit<ErColumn, "id">>,
): void {
  const table = tableOf(draft, nodeId);
  const column = table?.columns.find((c) => c.id === columnId);
  if (!table || !column) return;

  Object.assign(column, changes);

  // Una clave primaria no admite nulos; dejar las dos marcas en contra seria
  // un estado que ninguna base de datos acepta.
  if (column.primaryKey) column.nullable = false;

  resyncTableGeometry(draft, nodeId);
}

/** Mueve una columna una posicion arriba o abajo. */
export function moveColumn(
  draft: Document,
  nodeId: string,
  columnId: string,
  direction: -1 | 1,
): void {
  const table = tableOf(draft, nodeId);
  if (!table) return;

  const from = table.columns.findIndex((c) => c.id === columnId);
  const to = from + direction;
  if (from < 0 || to < 0 || to >= table.columns.length) return;

  const [column] = table.columns.splice(from, 1);
  table.columns.splice(to, 0, column!);
}

export function setTableAlias(draft: Document, nodeId: string, alias: string): void {
  const table = tableOf(draft, nodeId);
  if (table) table.alias = alias.trim() || undefined;
}

export function setTableNote(draft: Document, nodeId: string, note: string): void {
  const table = tableOf(draft, nodeId);
  if (table) table.note = note.trim() || undefined;
}
