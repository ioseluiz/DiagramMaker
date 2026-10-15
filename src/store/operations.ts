import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { EdgeKind } from "@/model/generated/EdgeKind";
import type { Node } from "@/model/generated/Node";
import type { NodeShape } from "@/model/generated/NodeShape";
import type { Side } from "@/model/generated/Side";
import { defaultSizeFor } from "@/model/shapes";

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
): string | null {
  const exists = (id: string) => draft.nodes.some((n) => n.id === id);
  if (!exists(source) || !exists(target)) return null;

  const id = newId("e");
  const edge: Edge = {
    id,
    source: { node: source, side: sides.source },
    target: { node: target, side: sides.target },
    kind,
    routing: "orthogonal",
    style: {},
  };

  draft.edges.push(edge);
  return id;
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
): void {
  const edge = draft.edges.find((e) => e.id === edgeId);
  if (!edge) return;
  if (!draft.nodes.some((n) => n.id === node)) return;

  edge[end].node = node;
  edge[end].side = side;
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
