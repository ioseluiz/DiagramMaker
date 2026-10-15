import type { Edge as FlowEdge, Node as FlowNode } from "@xyflow/react";

import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { EdgeKind } from "@/model/generated/EdgeKind";
import type { Node } from "@/model/generated/Node";
import type { Routing } from "@/model/generated/Routing";
import { defaultSizeFor, isContainer } from "@/model/shapes";
import { edgeStyle } from "@/model/style";

/**
 * Traduccion entre el IR y lo que React Flow espera.
 *
 * El IR guarda coordenadas **absolutas** siempre, porque es lo que necesitan el
 * auto-layout y el exportador de SVG. React Flow, en cambio, posiciona los
 * nodos anidados **relativos a su contenedor**. Toda esa conversion vive aqui y
 * en ningun otro sitio.
 */

export interface ShapeNodeData extends Record<string, unknown> {
  label: string;
  node: Node;
}

export type DiagramFlowNode = FlowNode<ShapeNodeData, "shape">;

const FLOW_EDGE_TYPE: Record<Routing, string> = {
  orthogonal: "smoothstep",
  straight: "straight",
  bezier: "default",
};

/** Marcadores SVG definidos en `EdgeMarkers.tsx`. */
const MARKERS: Record<EdgeKind, { start?: string; end?: string }> = {
  association: {},
  directed: { end: "dm-arrow-open" },
  dependency: { end: "dm-arrow-open" },
  inheritance: { end: "dm-triangle-hollow" },
  realization: { end: "dm-triangle-hollow" },
  composition: { start: "dm-diamond-filled" },
  aggregation: { start: "dm-diamond-hollow" },
  flow: { end: "dm-arrow-filled" },
};

const DASHED_KINDS: ReadonlySet<EdgeKind> = new Set<EdgeKind>([
  "dependency",
  "realization",
]);

export function isDashed(edge: Edge): boolean {
  return edgeStyle(edge).dashed ?? DASHED_KINDS.has(edge.kind);
}

/**
 * Origen del contenedor de un nodo. El IR guarda absolutas, asi que el origen
 * del padre es directamente su geometria; no hay que recorrer la cadena.
 */
function parentOrigin(doc: Document, node: Node): { x: number; y: number } {
  if (!node.parent) return { x: 0, y: 0 };

  const parent = doc.nodes.find((n) => n.id === node.parent);
  return parent?.geometry ?? { x: 0, y: 0 };
}

export function toFlowNodes(
  doc: Document,
  selection: ReadonlySet<string>,
): DiagramFlowNode[] {
  // Los contenedores van primero: React Flow exige que un padre aparezca en el
  // array antes que sus hijos.
  const ordered = [...doc.nodes].sort(
    (a, b) => Number(isContainer(b.shape)) - Number(isContainer(a.shape)),
  );

  return ordered.map((node) => {
    const [defaultWidth, defaultHeight] = defaultSizeFor(node.shape);
    const geometry = node.geometry ?? {
      x: 0,
      y: 0,
      width: defaultWidth,
      height: defaultHeight,
    };
    const origin = parentOrigin(doc, node);

    const flow: DiagramFlowNode = {
      id: node.id,
      type: "shape",
      position: { x: geometry.x - origin.x, y: geometry.y - origin.y },
      width: geometry.width,
      height: geometry.height,
      selected: selection.has(node.id),
      data: { label: node.label, node },
    };

    if (node.parent) {
      flow.parentId = node.parent;
      flow.extent = "parent";
    }

    return flow;
  });
}

export function toFlowEdges(doc: Document, selection: ReadonlySet<string>): FlowEdge[] {
  return doc.edges.map((edge) => {
    const markers = MARKERS[edge.kind];
    const style = edgeStyle(edge);

    return {
      id: edge.id,
      source: edge.source.node,
      target: edge.target.node,
      type: FLOW_EDGE_TYPE[edge.routing],
      label: edge.label ?? undefined,
      selected: selection.has(edge.id),
      markerStart: markers.start ? `url(#${markers.start})` : undefined,
      markerEnd: markers.end ? `url(#${markers.end})` : undefined,
      style: {
        stroke: style.stroke ?? "var(--edge-stroke)",
        strokeWidth: style.strokeWidth ?? 1.5,
        strokeDasharray: isDashed(edge) ? "6 4" : undefined,
      },
      data: { edge },
    } satisfies FlowEdge;
  });
}

/**
 * Convierte una posicion que viene de React Flow (relativa al contenedor) en la
 * posicion absoluta que guarda el IR.
 */
export function toAbsolute(
  doc: Document,
  nodeId: string,
  position: { x: number; y: number },
): { x: number; y: number } {
  const node = doc.nodes.find((n) => n.id === nodeId);
  if (!node) return position;

  const origin = parentOrigin(doc, node);
  return { x: position.x + origin.x, y: position.y + origin.y };
}
