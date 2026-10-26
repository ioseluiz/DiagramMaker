import ELK, { type ElkNode, type LayoutOptions } from "elkjs/lib/elk.bundled.js";

import type { Document } from "@/model/generated/Document";
import type { LayoutDirection } from "@/model/generated/LayoutDirection";
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

/** Como llama ELK a cada direccion. */
const ELK_DIRECTION: Record<LayoutDirection, string> = {
  right: "RIGHT",
  left: "LEFT",
  down: "DOWN",
  up: "UP",
};

/** Opciones de ELK. La direccion la pone el documento. */
const OPTIONS: LayoutOptions = {
  "elk.algorithm": "layered",
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
  const layoutOptions: LayoutOptions = {
    ...OPTIONS,
    "elk.direction": ELK_DIRECTION[doc.direction],
  };

  const byParent = new Map<string | null, ElkNode[]>();
  const contenedores = containersOf(doc);

  for (const node of doc.nodes) {
    // A un contenedor no se le manda tamano. Si se le manda, ELK lo respeta
    // y mete dentro lo que quepa: un paquete con seis tablas se quedaba en
    // los 220 x 160 de su forma y el contenido salia apelotonado. Sin
    // tamano, ELK lo calcula de lo que lleva dentro, que es lo que hace
    // falta.
    const medida = contenedores.has(node.id) ? undefined : sizeOf(node);

    const elkNode: ElkNode = {
      id: node.id,
      ...(medida ?? {}),
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
    layoutOptions,
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

/** Ids de los nodos que son padre de algun otro. */
function containersOf(doc: Document): Set<string> {
  const out = new Set<string>();
  for (const node of doc.nodes) {
    if (node.parent) out.add(node.parent);
  }
  return out;
}

/** Lo que ELK decide de cada nodo: donde va y, si lo calculo, cuanto mide. */
interface Placed {
  x: number;
  y: number;
  width?: number;
  height?: number;
}

/**
 * Geometria absoluta que devuelve ELK, aplanada por id de nodo.
 *
 * ELK da las posiciones relativas al contenedor de cada uno, y el documento
 * las guarda absolutas; de ahi los desplazamientos acumulados.
 */
function flatten(
  node: ElkNode,
  offsetX = 0,
  offsetY = 0,
  into = new Map<string, Placed>(),
): Map<string, Placed> {
  for (const child of node.children ?? []) {
    const x = offsetX + (child.x ?? 0);
    const y = offsetY + (child.y ?? 0);
    into.set(child.id, { x, y, width: child.width, height: child.height });
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

  const contenedores = containersOf(draft);

  for (const node of draft.nodes) {
    const placed = positions.get(node.id);
    if (!placed) continue;

    if (node.pinned && node.geometry) continue;

    // El tamano de un contenedor lo pone ELK, que es quien sabe cuanto ocupa
    // lo de dentro; el de una hoja sale de medir su texto.
    const medida =
      contenedores.has(node.id) && placed.width && placed.height
        ? { width: placed.width, height: placed.height }
        : sizeOf(node);

    node.geometry = {
      x: Math.round(placed.x),
      y: Math.round(placed.y),
      width: Math.round(medida.width),
      height: Math.round(medida.height),
    };
  }
}

export const __testing = { toElkGraph, flatten };
