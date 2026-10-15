import type { Edge as FlowEdge, Node as FlowNode } from "@xyflow/react";

import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { EdgeKind } from "@/model/generated/EdgeKind";
import type { Node } from "@/model/generated/Node";
import type { Point } from "@/model/generated/Point";
import type { Routing } from "@/model/generated/Routing";
import type { Side } from "@/model/generated/Side";
import { defaultSizeFor, isContainer } from "@/model/shapes";
import { tableSize } from "@/model/table";
import { edgeStyle } from "@/model/style";
import { TABLE_HEADER_HEIGHT, TABLE_ROW_HEIGHT, erTableOf } from "@/model/table";
import { findHops, type Hop } from "./crossings";
import { encodeHandle } from "./handles";
import { resolveSides, type SidePair } from "./sides";

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

/** Las tablas usan su propio componente; el resto comparte `ShapeNode`. */
export type DiagramFlowNode = FlowNode<ShapeNodeData, "shape" | "table">;

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

/** Compartido para que una arista sin cruces no cambie de identidad. */
const EMPTY_HOPS: readonly Hop[] = [];

const DASHED_KINDS: ReadonlySet<EdgeKind> = new Set<EdgeKind>([
  "dependency",
  "realization",
]);

export function isDashed(edge: Edge): boolean {
  return edgeStyle(edge).dashed ?? DASHED_KINDS.has(edge.kind);
}

/** Tamano de un nodo que todavia no tiene geometria. */
function fallbackSize(node: Node): { width: number; height: number } {
  if (node.shape === "table") return tableSize(node);

  const [width, height] = defaultSizeFor(node.shape);
  return { width, height };
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
    const isTable = node.shape === "table";

    // El tamano de una tabla sale de medir su texto, que no es gratis. Solo
    // se calcula cuando hace falta: un nodo ya colocado trae el suyo.
    const geometry = node.geometry ?? { x: 0, y: 0, ...fallbackSize(node) };
    const origin = parentOrigin(doc, node);

    const flow: DiagramFlowNode = {
      id: node.id,
      type: isTable ? "table" : "shape",
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

/**
 * Punto por donde una arista toca un nodo.
 *
 * Es el centro del lado, salvo que la arista este anclada a una columna: ahi
 * sale a la altura de su fila. Reproduce lo que React Flow coloca, y sirve
 * para saber por donde pasan las lineas sin esperar a que esten dibujadas.
 */
function attachPoint(
  doc: Document,
  nodeId: string,
  port: string | null | undefined,
  side: Side,
) {
  const node = doc.nodes.find((n) => n.id === nodeId);
  const g = node?.geometry;
  if (!g) return null;

  let y = g.y + g.height / 2;

  if (port && node) {
    const index = erTableOf(node)?.columns.findIndex((c) => c.id === port) ?? -1;
    if (index >= 0) {
      y = g.y + TABLE_HEADER_HEIGHT + index * TABLE_ROW_HEIGHT + TABLE_ROW_HEIGHT / 2;
    }
  }

  switch (side) {
    case "left":
      return { x: g.x, y };
    case "right":
      return { x: g.x + g.width, y };
    case "top":
      return { x: g.x + g.width / 2, y: g.y };
    case "bottom":
      return { x: g.x + g.width / 2, y: g.y + g.height };
  }
}

/**
 * Poligonal aproximada del trazado, para detectar cruces.
 *
 * No reproduce las curvas: un codo recto donde React Flow redondea la
 * esquina, y una recta donde dibuja una bezier. El salto es un arco de cinco
 * pixeles, asi que errar por uno o dos no se nota, y a cambio el calculo es
 * geometria elemental y se puede probar.
 */
function polylineFor(
  from: Point,
  to: Point,
  sides: SidePair,
  routing: Routing,
): Point[] {
  if (routing !== "orthogonal") return [from, to];

  const horizontal = sides.source === "left" || sides.source === "right";

  // El codo de React Flow gira a mitad de camino en el eje de salida.
  return horizontal
    ? [
        from,
        { x: (from.x + to.x) / 2, y: from.y },
        { x: (from.x + to.x) / 2, y: to.y },
        to,
      ]
    : [
        from,
        { x: from.x, y: (from.y + to.y) / 2 },
        { x: to.x, y: (from.y + to.y) / 2 },
        to,
      ];
}

/**
 * Lados y poligonal de cada arista.
 *
 * Los cruces se calculan de una vez para todas: una arista por su cuenta no
 * puede saber por donde pasan las demas.
 */
function edgeGeometry(doc: Document) {
  const geometryOf = (id: string) => doc.nodes.find((n) => n.id === id)?.geometry;

  const sidesById = new Map<string, SidePair>();
  const polylines = new Map<string, Point[]>();

  for (const edge of doc.edges) {
    const sides = resolveSides(
      { source: edge.source.side ?? undefined, target: edge.target.side ?? undefined },
      geometryOf(edge.source.node) ?? undefined,
      geometryOf(edge.target.node) ?? undefined,
      edge.source.node === edge.target.node,
    );
    sidesById.set(edge.id, sides);

    const from = attachPoint(doc, edge.source.node, edge.source.port, sides.source);
    const to = attachPoint(doc, edge.target.node, edge.target.port, sides.target);
    if (from && to) {
      polylines.set(edge.id, polylineFor(from, to, sides, edge.routing));
    }
  }

  return { sidesById, polylines };
}

/** Puentes de cada arista. Lo usa tambien la exportacion. */
export function hopsFor(doc: Document): Map<string, Hop[]> {
  return findHops(
    edgeGeometry(doc).polylines,
    doc.edges.map((e) => e.id),
  );
}

export function toFlowEdges(doc: Document, selection: ReadonlySet<string>): FlowEdge[] {
  const { sidesById, polylines } = edgeGeometry(doc);

  const hops = findHops(
    polylines,
    doc.edges.map((e) => e.id),
  );

  return doc.edges.map((edge) => {
    const markers = MARKERS[edge.kind];
    const style = edgeStyle(edge);

    // Sin handle explicito React Flow usa el primero que encuentre, que seria
    // siempre el de arriba. Hay que darselo resuelto.
    const sides = sidesById.get(edge.id)!;

    return {
      id: edge.id,
      source: edge.source.node,
      target: edge.target.node,
      sourceHandle: encodeHandle(sides.source, edge.source.port ?? undefined),
      targetHandle: encodeHandle(sides.target, edge.target.port ?? undefined),
      reconnectable: true,
      // Un solo tipo de arista para todo: `RelationEdge` elige el trazado
      // segun `routing` y ademas dibuja las cardinalidades de los extremos,
      // que React Flow por si solo no sabe poner.
      type: "relation",
      selected: selection.has(edge.id),
      markerStart: markers.start ? `url(#${markers.start})` : undefined,
      markerEnd: markers.end ? `url(#${markers.end})` : undefined,
      style: {
        stroke: style.stroke ?? "var(--edge-stroke)",
        strokeWidth: style.strokeWidth ?? 1.5,
        strokeDasharray: isDashed(edge) ? "6 4" : undefined,
      },
      data: {
        edge,
        routing: edge.routing,
        sourceLabel: edge.source.label ?? undefined,
        targetLabel: edge.target.label ?? undefined,
        centerLabel: edge.label ?? undefined,
        hops: hops.get(edge.id) ?? EMPTY_HOPS,
      },
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
