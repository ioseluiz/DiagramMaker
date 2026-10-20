import { current, isDraft } from "immer";

import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { EdgeKind } from "@/model/generated/EdgeKind";
import type { Node } from "@/model/generated/Node";
import type { NodeShape } from "@/model/generated/NodeShape";
import type { ErColumn } from "@/model/generated/ErColumn";
import type { ErTable } from "@/model/generated/ErTable";
import type { ClassBox } from "@/model/generated/ClassBox";
import type { ClassMember } from "@/model/generated/ClassMember";
import type { Side } from "@/model/generated/Side";
import type { BlockKind } from "@/model/generated/BlockKind";
import type { MessageKind } from "@/model/generated/MessageKind";
import type { NotePlacement } from "@/model/generated/NotePlacement";
import type { SequenceScript } from "@/model/generated/SequenceScript";
import type { Step } from "@/model/generated/Step";
import type { C4Level } from "@/model/c4";
import { classSize } from "@/model/classBox";
import { defaultSizeFor, labelPlacement } from "@/model/shapes";
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

  // En una secuencia lo borrado puede ser un participante o un paso del
  // guion; las dos cosas viven fuera de `nodes` y `edges`.
  if (draft.script) {
    for (const id of ids) removeStep(draft, id);
    pruneScript(draft);
  }
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

  // Una secuencia necesita guion desde el primer momento: es donde vive todo
  // lo que la define. Al cambiar a otro tipo se conserva, porque volver a
  // secuencia tiene que devolver el diagrama que habia.
  if (kind === "sequence") draft.script ??= { steps: [] };
}

/* --- Tablas entidad-relacion -------------------------------------------- */

/** Tabla de un nodo, si lo es. */
function tableOf(draft: Document, nodeId: string): ErTable | null {
  const node = draft.nodes.find((n) => n.id === nodeId);
  return node?.data.type === "er" ? node.data : null;
}

/* --- Cajas de clase ------------------------------------------------------ */

export interface AddClassOptions {
  label?: string;
  x: number;
  y: number;
}

/**
 * Anade una clase con un atributo y una operacion dentro.
 *
 * Nace con algo igual que una tabla nace con su `id`: una caja con los tres
 * compartimentos vacios no deja ver que es una clase, y lo primero que hace
 * cualquiera es anadir un miembro.
 */
export function addClass(draft: Document, options: AddClassOptions): string {
  const id = newId("k");

  const node: Node = {
    id,
    label: options.label ?? "Clase",
    shape: "class",
    pinned: true,
    style: {},
    data: {
      type: "class",
      attributes: [
        {
          id: newId("a"),
          name: "atributo",
          visibility: "private",
          typeName: "String",
          parameters: "",
        },
      ],
      operations: [
        {
          id: newId("o"),
          name: "operacion",
          visibility: "public",
          typeName: "",
          parameters: "",
        },
      ],
    },
  };

  draft.nodes.push(node);
  node.geometry = { x: options.x, y: options.y, ...classSize(node) };

  return id;
}

/** Caja de clase de un nodo del borrador, si lo es. */
function classOf(draft: Document, nodeId: string): ClassBox | null {
  const node = draft.nodes.find((n) => n.id === nodeId);
  return node?.data.type === "class" ? node.data : null;
}

/**
 * Reajusta la caja tras cambiar los miembros.
 *
 * El alto es derivado —tres compartimentos y una fila por miembro— y el
 * ancho solo crece: encogerla porque se borro el miembro mas largo seria
 * pelearse con quien la habia ensanchado a mano.
 */
function resyncClassGeometry(draft: Document, nodeId: string): void {
  const node = draft.nodes.find((n) => n.id === nodeId);
  if (!node || node.data.type !== "class" || !node.geometry) return;

  const measured = classSize(node);
  node.geometry.height = measured.height;
  node.geometry.width = Math.max(node.geometry.width, measured.width);
}

export function addClassMember(
  draft: Document,
  nodeId: string,
  compartment: "attributes" | "operations",
): string | undefined {
  const box = classOf(draft, nodeId);
  if (!box) return undefined;

  const id = newId(compartment === "attributes" ? "a" : "o");
  box[compartment].push({
    id,
    name: compartment === "attributes" ? "atributo" : "operacion",
    visibility: compartment === "attributes" ? "private" : "public",
    typeName: "",
    parameters: "",
  });

  resyncClassGeometry(draft, nodeId);
  return id;
}

export function removeClassMember(
  draft: Document,
  nodeId: string,
  memberId: string,
): void {
  const box = classOf(draft, nodeId);
  if (!box) return;

  box.attributes = box.attributes.filter((m) => m.id !== memberId);
  box.operations = box.operations.filter((m) => m.id !== memberId);

  resyncClassGeometry(draft, nodeId);
}

export function updateClassMember(
  draft: Document,
  nodeId: string,
  memberId: string,
  patch: Partial<ClassMember>,
): void {
  const box = classOf(draft, nodeId);
  if (!box) return;

  for (const list of [box.attributes, box.operations]) {
    const member = list.find((m) => m.id === memberId);
    if (member) {
      Object.assign(member, patch);
      resyncClassGeometry(draft, nodeId);
      return;
    }
  }
}

/** Sube o baja un miembro dentro de su compartimento. */
export function moveClassMember(
  draft: Document,
  nodeId: string,
  memberId: string,
  delta: number,
): void {
  const box = classOf(draft, nodeId);
  if (!box) return;

  for (const list of [box.attributes, box.operations]) {
    const from = list.findIndex((m) => m.id === memberId);
    if (from < 0) continue;

    const to = from + delta;
    if (to < 0 || to >= list.length) return;

    const [member] = list.splice(from, 1);
    list.splice(to, 0, member!);
    return;
  }
}

export function setClassStereotype(
  draft: Document,
  nodeId: string,
  stereotype: string,
): void {
  const box = classOf(draft, nodeId);
  if (!box) return;

  box.stereotype = stereotype.trim() ? stereotype : undefined;
  resyncClassGeometry(draft, nodeId);
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

/**
 * Tipo de una columna recien creada.
 *
 * No se deja vacio porque el DBML que se genera entonces es `columna` a
 * secas, y en dbdiagram.io el tipo es obligatorio: el codigo exportado no se
 * podria pegar alli. `varchar` es el tipo mas comun y se cambia en un campo.
 */
const DEFAULT_COLUMN_TYPE = "varchar";

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
    dataType: DEFAULT_COLUMN_TYPE,
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

/* --- Operaciones de menu contextual ------------------------------------- */

/** Cuanto se desplaza una copia respecto al original. */
const DUPLICATE_OFFSET = 24;

/**
 * Duplica un nodo y lo que contiene.
 *
 * Arrastra a los descendientes: duplicar un paquete y obtener una caja vacia
 * no es lo que nadie espera. Las relaciones **no** se copian — una copia de
 * una tabla no hereda sus claves ajenas, que apuntarian a las mismas
 * columnas del original.
 *
 * Las columnas reciben ids nuevos: son estables dentro de su tabla, y dos
 * tablas no pueden compartirlos.
 */
export function duplicateNode(draft: Document, nodeId: string): string | null {
  const original = draft.nodes.find((n) => n.id === nodeId);
  if (!original) return null;

  // Recoger el nodo y toda su descendencia.
  const family = [original];
  for (let i = 0; i < family.length; i++) {
    const parent = family[i]!;
    for (const node of draft.nodes) {
      if (node.parent === parent.id) family.push(node);
    }
  }

  const renamed = new Map<string, string>();
  for (const node of family) renamed.set(node.id, newId("n"));

  for (const node of family) {
    // Un borrador de immer es un proxy y `structuredClone` no sabe copiarlo;
    // `current` devuelve la instantanea plana que hay detras.
    const copy = structuredClone(isDraft(node) ? current(node) : node) as Node;
    copy.id = renamed.get(node.id)!;

    // Todos se desplazan lo mismo: la geometria es absoluta, asi que mover
    // padre e hijos por igual conserva sus posiciones relativas.
    if (copy.geometry) {
      copy.geometry.x += DUPLICATE_OFFSET;
      copy.geometry.y += DUPLICATE_OFFSET;
    }

    // La copia raiz se queda en el mismo contenedor que el original; sus
    // descendientes cuelgan de la copia correspondiente.
    copy.parent = node.id === nodeId ? node.parent : renamed.get(node.parent ?? "");

    if (copy.data.type === "er") {
      copy.data.columns = copy.data.columns.map((column) => ({
        ...column,
        id: newId("c"),
        // La copia no hereda las claves ajenas: sus relaciones no se copian.
        foreignKey: false,
      }));
    }

    draft.nodes.push(copy);
  }

  return renamed.get(nodeId)!;
}

/**
 * Invierte el sentido de una relacion.
 *
 * Cambia los dos extremos enteros —nodo, columna, lado y cardinalidad— de
 * modo que `posts.user_id * → 1 users.id` pasa a `users.id 1 → * posts.user_id`:
 * la misma relacion leida al reves, no una distinta.
 */
export function reverseEdge(draft: Document, edgeId: string): void {
  const edge = draft.edges.find((e) => e.id === edgeId);
  if (!edge) return;

  const { source, target } = edge;
  edge.source = target;
  edge.target = source;
}

/** Fija o quita el relleno de un nodo. `null` devuelve el color del tema. */
export function setNodeFill(
  draft: Document,
  nodeId: string,
  fill: string | null,
): void {
  const node = draft.nodes.find((n) => n.id === nodeId);
  if (!node) return;

  // Borde y texto vuelven a los del tema. Los tintes de la paleta estan
  // pensados para leerse asi; si el nodo venia de un nivel C4, conservar su
  // texto blanco sobre un tinte palido lo dejaria invisible.
  node.style = {
    ...node.style,
    fill: fill ?? undefined,
    stroke: undefined,
    textColor: undefined,
  };
}

/**
 * Aplica —o quita— el estilo de un nivel del modelo C4.
 *
 * Escribe relleno, borde y color de texto de una vez porque los tres van
 * juntos: el relleno de C4 es solido y saturado, y con el texto del tema no
 * se leeria. Queda como un unico paso de deshacer.
 */
export function setNodeC4Level(
  draft: Document,
  nodeId: string,
  level: C4Level | null,
): void {
  const node = draft.nodes.find((n) => n.id === nodeId);
  if (!node) return;

  // El color de texto de un nivel esta calculado para leerse *sobre* su
  // relleno. En las formas cuya etiqueta va fuera de la caja —el actor la
  // lleva debajo— el texto cae sobre el fondo del lienzo, donde el blanco de
  // C4 desapareceria; alli manda el color del tema.
  const onFill = labelPlacement(node.shape) !== "below";

  node.style = {
    ...node.style,
    fill: level?.fill,
    stroke: level?.stroke,
    textColor: level && onFill ? level.textColor : undefined,
  };
}

/* --- Agrupar y desagrupar ------------------------------------------------ */

/** Hueco entre el borde del grupo y lo que contiene. */
const GROUP_PADDING = 24;
/** Alto de la solapa del paquete; espejo de `shape_geometry.rs`. */
const GROUP_TAB_HEIGHT = 22;

/**
 * Envuelve los nodos indicados en un contenedor.
 *
 * Solo toma los que estan en la raiz: mover a un grupo un nodo que ya vive
 * dentro de otro lo sacaria de su sitio sin haberlo pedido.
 */
export function groupNodes(
  draft: Document,
  nodeIds: readonly string[],
  label = "grupo",
): string | null {
  const members = draft.nodes.filter(
    (n) => nodeIds.includes(n.id) && !n.parent && n.geometry,
  );
  if (members.length === 0) return null;

  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;

  for (const node of members) {
    const g = node.geometry!;
    left = Math.min(left, g.x);
    top = Math.min(top, g.y);
    right = Math.max(right, g.x + g.width);
    bottom = Math.max(bottom, g.y + g.height);
  }

  const id = newId("g");
  const node: Node = {
    id,
    label,
    shape: "package",
    pinned: true,
    style: {},
    data: { type: "plain" },
    geometry: {
      x: left - GROUP_PADDING,
      // Arriba hace falta sitio extra: la solapa lleva el nombre del grupo.
      y: top - GROUP_PADDING - GROUP_TAB_HEIGHT,
      width: right - left + GROUP_PADDING * 2,
      height: bottom - top + GROUP_PADDING * 2 + GROUP_TAB_HEIGHT,
    },
  };

  // El contenedor va primero: el lienzo y el exportador esperan ver al padre
  // antes que a sus hijos.
  draft.nodes.unshift(node);
  for (const member of members) member.parent = id;

  return id;
}

/** Saca a los hijos de un contenedor y lo elimina. */
export function ungroupNode(draft: Document, groupId: string): void {
  const group = draft.nodes.find((n) => n.id === groupId);
  if (!group) return;

  const parent = group.parent;
  for (const node of draft.nodes) {
    // Los hijos suben un nivel: al contenedor del grupo, o a la raiz.
    if (node.parent === groupId) node.parent = parent;
  }

  draft.nodes = draft.nodes.filter((n) => n.id !== groupId);
  draft.edges = draft.edges.filter(
    (e) => e.source.node !== groupId && e.target.node !== groupId,
  );
}

/**
 * Cambia el contenedor de un nodo. `null` lo devuelve a la raiz.
 *
 * Rechaza meter un nodo dentro de si mismo o de su propia descendencia: eso
 * produciria un ciclo de anidamiento, que es justo lo que `validate` marca
 * como error y lo que cuelga al auto-layout jerarquico.
 */
export function setNodeParent(
  draft: Document,
  nodeId: string,
  parentId: string | null,
): void {
  const node = draft.nodes.find((n) => n.id === nodeId);
  if (!node || nodeId === parentId) return;

  if (parentId) {
    if (!draft.nodes.some((n) => n.id === parentId)) return;

    let cursor: string | undefined = parentId;
    for (let guard = 0; cursor && guard <= draft.nodes.length; guard++) {
      if (cursor === nodeId) return;
      cursor = draft.nodes.find((n) => n.id === cursor)?.parent ?? undefined;
    }
  }

  node.parent = parentId ?? undefined;
}

/* --- Diagramas de secuencia ---------------------------------------------- */

/**
 * Un diagrama de secuencia se edita por su guion, no por coordenadas: estas
 * operaciones mueven pasos dentro de una lista ordenada y el reparto decide
 * donde cae cada cosa. Por eso ninguna toca `geometry`.
 */

/** Guion del documento, creandolo si hacia falta. */
function scriptDraft(draft: Document): SequenceScript {
  draft.script ??= { steps: [] };
  return draft.script;
}

/** Lista que contiene un paso, y su posicion. */
function siblingsOf(
  steps: Step[],
  id: string,
): { list: Step[]; position: number } | undefined {
  const position = steps.findIndex((s) => s.id === id);
  if (position >= 0) return { list: steps, position };

  for (const step of steps) {
    if (step.type !== "block") continue;
    for (const branch of step.branches) {
      const found = siblingsOf(branch.steps, id);
      if (found) return found;
    }
  }

  return undefined;
}

/** Todos los pasos, entrando en los marcos. */
function allSteps(steps: Step[]): Step[] {
  return steps.flatMap((step) =>
    step.type === "block"
      ? [step, ...step.branches.flatMap((b) => allSteps(b.steps))]
      : [step],
  );
}

export function addParticipant(
  draft: Document,
  label: string,
  shape: NodeShape = "rectangle",
): string {
  const id = newId("p");

  // Sin geometria: en una secuencia la posicion sale del orden, y el orden
  // de los participantes es el de esta lista.
  draft.nodes.push({
    id,
    label,
    shape,
    pinned: false,
    style: {},
    data: { type: "plain" },
  });

  return id;
}

/** Mueve un participante a izquierda o derecha en el diagrama. */
export function moveParticipant(draft: Document, nodeId: string, delta: number): void {
  const from = draft.nodes.findIndex((n) => n.id === nodeId);
  if (from < 0) return;

  const to = from + delta;
  if (to < 0 || to >= draft.nodes.length) return;

  const [node] = draft.nodes.splice(from, 1);
  draft.nodes.splice(to, 0, node!);
}

export interface AddMessageOptions {
  from: string;
  to: string;
  kind?: MessageKind;
  text?: string;
  /** Detras de que paso se inserta. Al final si no se dice. */
  after?: string;
}

export function addMessage(draft: Document, options: AddMessageOptions): string {
  const id = newId("m");
  const message: Step = {
    type: "message",
    id,
    from: options.from,
    to: options.to,
    kind: options.kind ?? "sync",
    text: options.text ?? "",
    activate: false,
    deactivate: false,
  };

  insert(draft, message, options.after);
  return id;
}

export function addSequenceBlock(
  draft: Document,
  kind: BlockKind,
  label = "",
  after?: string,
): string {
  const id = newId("b");

  insert(
    draft,
    {
      type: "block",
      id,
      kind,
      branches: [{ id: newId("r"), label, steps: [] }],
    },
    after,
  );

  return id;
}

export function addSequenceNote(
  draft: Document,
  participants: string[],
  placement: NotePlacement,
  text: string,
  after?: string,
): string {
  const id = newId("n");
  insert(draft, { type: "note", id, placement, participants, text }, after);
  return id;
}

/** Mete un paso detras de otro, o al final del guion. */
function insert(draft: Document, step: Step, after?: string): void {
  const script = scriptDraft(draft);

  const target = after ? siblingsOf(script.steps, after) : undefined;
  if (target) {
    target.list.splice(target.position + 1, 0, step);
    return;
  }

  script.steps.push(step);
}

/** Anade una rama a un marco: el `else` de un `alt`. */
export function addBranch(
  draft: Document,
  blockId: string,
  label = "",
): string | undefined {
  const block = allSteps(scriptDraft(draft).steps).find(
    (s) => s.id === blockId && s.type === "block",
  );
  if (block?.type !== "block") return undefined;

  const id = newId("r");
  block.branches.push({ id, label, steps: [] });
  return id;
}

export function removeStep(draft: Document, id: string): void {
  const script = scriptDraft(draft);

  const found = siblingsOf(script.steps, id);
  if (found) {
    found.list.splice(found.position, 1);
    return;
  }

  // Tambien puede ser una rama: borrarla se lleva lo que contenga, pero
  // nunca la ultima, que dejaria el marco sin cuerpo.
  for (const step of allSteps(script.steps)) {
    if (step.type !== "block") continue;
    const position = step.branches.findIndex((b) => b.id === id);
    if (position >= 0 && step.branches.length > 1) {
      step.branches.splice(position, 1);
      return;
    }
  }
}

/**
 * Sube o baja un paso entre sus hermanos.
 *
 * No lo saca de su marco: salir de un `alt` arrastrando cambiaria lo que el
 * diagrama dice, y para eso esta borrar y volver a crear.
 */
export function moveStep(draft: Document, id: string, delta: number): void {
  const found = siblingsOf(scriptDraft(draft).steps, id);
  if (!found) return;

  const to = found.position + delta;
  if (to < 0 || to >= found.list.length) return;

  const [step] = found.list.splice(found.position, 1);
  found.list.splice(to, 0, step!);
}

/** Texto de un mensaje o de una nota. */
export function setStepText(draft: Document, id: string, text: string): void {
  const step = allSteps(scriptDraft(draft).steps).find((s) => s.id === id);
  if (step?.type === "message" || step?.type === "note") step.text = text;
}

export function setMessageKind(draft: Document, id: string, kind: MessageKind): void {
  const step = allSteps(scriptDraft(draft).steps).find((s) => s.id === id);
  if (step?.type === "message") step.kind = kind;
}

export function setMessageEndpoint(
  draft: Document,
  id: string,
  end: "from" | "to",
  node: string,
): void {
  const step = allSteps(scriptDraft(draft).steps).find((s) => s.id === id);
  if (step?.type === "message") step[end] = node;
}

export function setMessageActivation(
  draft: Document,
  id: string,
  which: "activate" | "deactivate",
  value: boolean,
): void {
  const step = allSteps(scriptDraft(draft).steps).find((s) => s.id === id);
  if (step?.type === "message") step[which] = value;
}

/** Condicion de una rama. La primera se rotula en la esquina del marco. */
export function setBranchLabel(draft: Document, branchId: string, label: string): void {
  for (const step of allSteps(scriptDraft(draft).steps)) {
    if (step.type !== "block") continue;
    const branch = step.branches.find((b) => b.id === branchId);
    if (branch) {
      branch.label = label;
      return;
    }
  }
}

export function setNotePlacement(
  draft: Document,
  id: string,
  placement: NotePlacement,
): void {
  const step = allSteps(scriptDraft(draft).steps).find((s) => s.id === id);
  if (step?.type === "note") step.placement = placement;
}

/**
 * Quita del guion los pasos que apuntan a participantes que ya no existen.
 *
 * Un mensaje huerfano no se dibuja —el reparto lo ignora— pero seguiria en el
 * archivo y reapareceria al volver a crear un participante con ese id.
 */
export function pruneScript(draft: Document): void {
  const script = draft.script;
  if (!script) return;

  const alive = new Set(draft.nodes.map((n) => n.id));

  const keep = (steps: Step[]): Step[] =>
    steps
      .filter((step) => {
        if (step.type === "message") return alive.has(step.from) && alive.has(step.to);
        if (step.type === "note") return step.participants.every((p) => alive.has(p));
        return true;
      })
      .map((step) =>
        step.type === "block"
          ? {
              ...step,
              branches: step.branches.map((b) => ({ ...b, steps: keep(b.steps) })),
            }
          : step,
      );

  script.steps = keep(script.steps);
}
