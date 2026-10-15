import ELK, { type ElkNode, type LayoutOptions } from "elkjs/lib/elk.bundled.js";

import type { Document } from "@/model/generated/Document";
import { defaultSizeFor } from "@/model/shapes";
import { tableSize } from "@/model/table";

/**
 * Auto-layout de los nodos sin colocar.
 *
 * Vive en el frontend y no en Rust porque el tamano de cada caja sale de medir
 * su texto, y medir texto es cosa del navegador. ELK recibe cajas ya medidas y
 * devuelve posiciones.
 *
 * Los nodos anclados (`pinned`) no se mueven: el usuario ya decidio donde van.
 */

const elk = new ELK();

/** Opciones de ELK. Capas de izquierda a derecha, que es como se lee un ER. */
const OPTIONS: LayoutOptions = {
  "elk.algorithm": "layered",
  "elk.direction": "RIGHT",
  "elk.layered.spacing.nodeNodeBetweenLayers": "90",
  "elk.spacing.nodeNode": "48",
  "elk.layered.nodePlacement.strategy": "NETWORK_SIMPLEX",
  "elk.layered.considerModelOrder.strategy": "NODES_AND_EDGES",
  "elk.hierarchyHandling": "INCLUDE_CHILDREN",
  "elk.padding": "[top=40,left=20,bottom=20,right=20]",
};

/** Tamano que debe tener un nodo, segun su forma y sus datos. */
export function sizeOf(node: Document["nodes"][number]): {
  width: number;
  height: number;
} {
  if (node.geometry) {
    return { width: node.geometry.width, height: node.geometry.height };
  }

  if (node.data.type === "er") return tableSize(node);

  const [width, height] = defaultSizeFor(node.shape);
  return { width, height };
}

/** Convierte el documento al grafo jerarquico que espera ELK. */
function toElkGraph(doc: Document): ElkNode {
  const byParent = new Map<string | null, ElkNode[]>();

  for (const node of doc.nodes) {
    const { width, height } = sizeOf(node);
    const elkNode: ElkNode = {
      id: node.id,
      width,
      height,
      // Un nodo anclado conserva su sitio: ELK lo trata como fijo.
      ...(node.pinned && node.geometry
        ? { x: node.geometry.x, y: node.geometry.y }
        : {}),
    };

    const key = node.parent ?? null;
    const siblings = byParent.get(key) ?? [];
    siblings.push(elkNode);
    byParent.set(key, siblings);
  }

  // Colgar cada grupo de hijos de su contenedor.
  for (const [parent, children] of byParent) {
    if (parent === null) continue;
    const container = doc.nodes.find((n) => n.id === parent);
    if (!container) continue;

    const all = [...byParent.values()].flat();
    const elkParent = all.find((n) => n.id === parent);
    if (elkParent) elkParent.children = children;
  }

  return {
    id: "root",
    layoutOptions: OPTIONS,
    children: byParent.get(null) ?? [],
    edges: doc.edges
      // ELK no entiende los bucles sobre un mismo nodo; los coloca mal y
      // ademas no aportan nada al orden de las capas.
      .filter((e) => e.source.node !== e.target.node)
      .map((e) => ({
        id: e.id,
        sources: [e.source.node],
        targets: [e.target.node],
      })),
  };
}

/** Posiciones absolutas que devuelve ELK, aplanadas por id de nodo. */
function flatten(
  node: ElkNode,
  offsetX = 0,
  offsetY = 0,
  into = new Map<string, { x: number; y: number }>(),
): Map<string, { x: number; y: number }> {
  for (const child of node.children ?? []) {
    const x = offsetX + (child.x ?? 0);
    const y = offsetY + (child.y ?? 0);
    into.set(child.id, { x, y });
    flatten(child, x, y, into);
  }
  return into;
}

/**
 * Coloca los nodos que no tienen geometria.
 *
 * Muta el borrador que se le pase, para poder usarse dentro de un `edit` del
 * store y quedar como un solo paso de deshacer.
 */
export async function applyAutoLayout(draft: Document): Promise<void> {
  if (draft.nodes.length === 0) return;

  const positions = flatten(await elk.layout(toElkGraph(draft)));

  for (const node of draft.nodes) {
    const position = positions.get(node.id);
    if (!position) continue;

    const { width, height } = sizeOf(node);

    if (node.pinned && node.geometry) continue;

    node.geometry = {
      x: Math.round(position.x),
      y: Math.round(position.y),
      width,
      height,
    };
  }
}

export const __testing = { toElkGraph, flatten };
