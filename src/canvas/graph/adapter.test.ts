import { describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { Node } from "@/model/generated/Node";
import type { NodeShape } from "@/model/generated/NodeShape";
import { isDashed, toAbsolute, toFlowEdges, toFlowNodes } from "./adapter";

const NONE: ReadonlySet<string> = new Set();

function node(
  id: string,
  x: number,
  y: number,
  extra: Partial<Node> = {},
  shape: NodeShape = "rectangle",
): Node {
  return {
    id,
    label: id,
    shape,
    geometry: { x, y, width: 160, height: 60 },
    pinned: false,
    data: { type: "plain" },
    ...extra,
  };
}

function doc(nodes: Node[], edges: Edge[] = []): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "architecture",
    meta: { title: "t", created: 0, modified: 0 },
    nodes,
    edges,
    layout: "manual",
  };
}

describe("toFlowNodes", () => {
  it("pasa las coordenadas tal cual cuando no hay anidamiento", () => {
    const [flow] = toFlowNodes(doc([node("a", 40, 80)]), NONE);

    expect(flow!.position).toEqual({ x: 40, y: 80 });
    expect(flow!.parentId).toBeUndefined();
  });

  it("convierte la posicion del hijo a relativa respecto al contenedor", () => {
    const d = doc([
      node("pkg", 100, 100, {}, "package"),
      node("a", 160, 140, { parent: "pkg" }),
    ]);

    const child = toFlowNodes(d, NONE).find((n) => n.id === "a")!;

    expect(child.position).toEqual({ x: 60, y: 40 });
    expect(child.parentId).toBe("pkg");
    expect(child.extent).toBe("parent");
  });

  it("coloca los contenedores antes que sus hijos, como exige React Flow", () => {
    const d = doc([
      node("a", 160, 140, { parent: "pkg" }),
      node("pkg", 100, 100, {}, "package"),
    ]);

    expect(toFlowNodes(d, NONE).map((n) => n.id)).toEqual(["pkg", "a"]);
  });

  it("da una posicion y un tamano utilizables a un nodo sin geometria", () => {
    const sinGeometria = node("a", 0, 0);
    delete sinGeometria.geometry;

    const [flow] = toFlowNodes(doc([sinGeometria]), NONE);

    expect(flow!.position).toEqual({ x: 0, y: 0 });
    expect(flow!.width).toBe(160);
    expect(flow!.height).toBe(60);
  });

  it("marca como seleccionados los nodos de la seleccion", () => {
    const d = doc([node("a", 0, 0), node("b", 300, 0)]);

    const flow = toFlowNodes(d, new Set(["b"]));

    expect(flow.find((n) => n.id === "a")!.selected).toBe(false);
    expect(flow.find((n) => n.id === "b")!.selected).toBe(true);
  });
});

describe("toAbsolute", () => {
  it("es la inversa exacta de la conversion a relativa", () => {
    const d = doc([
      node("pkg", 100, 100, {}, "package"),
      node("a", 160, 140, { parent: "pkg" }),
    ]);

    const relative = toFlowNodes(d, NONE).find((n) => n.id === "a")!.position;

    expect(toAbsolute(d, "a", relative)).toEqual({ x: 160, y: 140 });
  });

  it("deja intacta la posicion de un nodo raiz", () => {
    const d = doc([node("a", 0, 0)]);

    expect(toAbsolute(d, "a", { x: 25, y: 35 })).toEqual({ x: 25, y: 35 });
  });

  it("devuelve la posicion recibida si el nodo no existe", () => {
    expect(toAbsolute(doc([]), "fantasma", { x: 5, y: 5 })).toEqual({ x: 5, y: 5 });
  });
});

describe("toFlowEdges", () => {
  function edge(extra: Partial<Edge> = {}): Edge {
    return {
      id: "e1",
      source: { node: "a" },
      target: { node: "b" },
      kind: "association",
      routing: "orthogonal",
      ...extra,
    };
  }

  const two = [node("a", 0, 0), node("b", 300, 0)];

  it("traduce el ruteo al tipo de arista de React Flow", () => {
    const d = doc(two, [
      edge({ id: "e1", routing: "orthogonal" }),
      edge({ id: "e2", routing: "straight" }),
      edge({ id: "e3", routing: "bezier" }),
    ]);

    expect(toFlowEdges(d, NONE).map((e) => e.type)).toEqual([
      "smoothstep",
      "straight",
      "default",
    ]);
  });

  it("pone el triangulo hueco al final de una herencia", () => {
    const [flow] = toFlowEdges(doc(two, [edge({ kind: "inheritance" })]), NONE);

    expect(flow!.markerEnd).toBe("url(#dm-triangle-hollow)");
    expect(flow!.markerStart).toBeUndefined();
  });

  it("pone el rombo relleno al inicio de una composicion", () => {
    const [flow] = toFlowEdges(doc(two, [edge({ kind: "composition" })]), NONE);

    expect(flow!.markerStart).toBe("url(#dm-diamond-filled)");
    expect(flow!.markerEnd).toBeUndefined();
  });

  it("nunca engancha las dos puntas arriba por defecto", () => {
    // Regresion: sin `sourceHandle`/`targetHandle` React Flow usaba el primer
    // handle declarado, que era el superior, y todas las aristas nacian y
    // morian en el vertice de arriba.
    const d = doc(two, [edge()]);

    const [flow] = toFlowEdges(d, NONE);

    expect(flow!.sourceHandle).toBe("right");
    expect(flow!.targetHandle).toBe("left");
  });

  it("el lado guardado por el usuario manda sobre la geometria", () => {
    const d = doc(two, [
      edge({
        source: { node: "a", side: "bottom" },
        target: { node: "b", side: "top" },
      }),
    ]);

    const [flow] = toFlowEdges(d, NONE);

    expect(flow!.sourceHandle).toBe("bottom");
    expect(flow!.targetHandle).toBe("top");
  });

  it("los lados se recalculan al mover las cajas si no estan fijados", () => {
    const apiladas = [node("a", 0, 0), node("b", 0, 300)];

    const [flow] = toFlowEdges(doc(apiladas, [edge()]), NONE);

    expect(flow!.sourceHandle).toBe("bottom");
    expect(flow!.targetHandle).toBe("top");
  });

  it("las aristas se pueden reenganchar arrastrando la punta", () => {
    const [flow] = toFlowEdges(doc(two, [edge()]), NONE);

    expect(flow!.reconnectable).toBe(true);
  });

  it("una asociacion simple no lleva marcadores", () => {
    const [flow] = toFlowEdges(doc(two, [edge()]), NONE);

    expect(flow!.markerStart).toBeUndefined();
    expect(flow!.markerEnd).toBeUndefined();
  });
});

describe("isDashed", () => {
  const base: Edge = {
    id: "e1",
    source: { node: "a" },
    target: { node: "b" },
    kind: "association",
    routing: "orthogonal",
  };

  it("dependencia y realizacion van punteadas por defecto", () => {
    expect(isDashed({ ...base, kind: "dependency" })).toBe(true);
    expect(isDashed({ ...base, kind: "realization" })).toBe(true);
    expect(isDashed({ ...base, kind: "association" })).toBe(false);
  });

  it("el estilo explicito manda sobre el defecto del tipo", () => {
    expect(isDashed({ ...base, kind: "dependency", style: { dashed: false } })).toBe(
      false,
    );
    expect(isDashed({ ...base, kind: "association", style: { dashed: true } })).toBe(
      true,
    );
  });
});
