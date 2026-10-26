import { describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { Edge } from "@/model/generated/Edge";
import type { Node } from "@/model/generated/Node";
import type { NodeShape } from "@/model/generated/NodeShape";
import { DEFAULT_STROKE_WIDTH } from "@/store/operations";
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
    direction: "right",
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

  it("el grosor elegido llega al trazo", () => {
    const d = doc(two, [edge({ style: { strokeWidth: 4 } })]);
    const [flow] = toFlowEdges(d, NONE);

    expect(flow!.style?.strokeWidth).toBe(4);
  });

  it("y sin elegir ninguno usa el mismo que Rust", () => {
    // `render::svg` escribe `unwrap_or(1.5)`. Si los dos numeros se
    // separaran, una relacion se veria de un grosor en pantalla y saldria de
    // otro en el PNG, que es justo lo que este proyecto no quiere.
    const [flow] = toFlowEdges(doc(two, [edge()]), NONE);

    expect(flow!.style?.strokeWidth).toBe(DEFAULT_STROKE_WIDTH);
  });

  it("todas las aristas usan el mismo componente y el ruteo viaja en los datos", () => {
    // `RelationEdge` elige el trazado y ademas dibuja las cardinalidades,
    // que React Flow por si solo no sabe poner en los extremos.
    const d = doc(two, [
      edge({ id: "e1", routing: "orthogonal" }),
      edge({ id: "e2", routing: "straight" }),
      edge({ id: "e3", routing: "bezier" }),
    ]);

    const flow = toFlowEdges(d, NONE);

    expect(flow.map((e) => e.type)).toEqual(["relation", "relation", "relation"]);
    expect(flow.map((e) => e.data!.routing)).toEqual([
      "orthogonal",
      "straight",
      "bezier",
    ]);
  });

  it("las cardinalidades de los extremos llegan al lienzo", () => {
    // Antes solo salian al exportar: en pantalla no se veia si una relacion
    // era de uno a muchos o de muchos a muchos.
    const d = doc(two, [
      edge({
        source: { node: "a", label: "*" },
        target: { node: "b", label: "1" },
        label: "pertenece a",
      }),
    ]);

    const [flow] = toFlowEdges(d, NONE);

    expect(flow!.data).toMatchObject({
      sourceLabel: "*",
      targetLabel: "1",
      centerLabel: "pertenece a",
    });
  });

  it("una arista anclada a columnas usa los conectores de fila", () => {
    const d = doc(two, [
      edge({
        source: { node: "a", port: "c_1" },
        target: { node: "b", port: "c_2" },
      }),
    ]);

    const [flow] = toFlowEdges(d, NONE);

    expect(flow!.sourceHandle).toBe("c_1@right");
    expect(flow!.targetHandle).toBe("c_2@left");
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

describe("seleccion de relaciones", () => {
  const two = [node("a", 0, 0), node("b", 300, 0)];
  const plain = (): Edge => ({
    id: "e1",
    source: { node: "a" },
    target: { node: "b" },
    kind: "association",
    routing: "orthogonal",
  });

  it("marca la arista seleccionada para que el componente lo sepa", () => {
    const d = doc(two, [plain()]);

    expect(toFlowEdges(d, new Set(["e1"]))[0]!.selected).toBe(true);
    expect(toFlowEdges(d, NONE)[0]!.selected).toBe(false);
  });

  it("el trazo llega como estilo en linea, asi que el color de seleccion no puede ser CSS", () => {
    // Fija la razon por la que `RelationEdge` resuelve la seleccion en el
    // componente: un estilo en linea gana a cualquier regla.
    const [flow] = toFlowEdges(doc(two, [plain()]), new Set(["e1"]));

    expect(flow!.style).toHaveProperty("stroke");
  });
});

describe("orden de pintado de los contenedores", () => {
  it("deja los contenedores por debajo de las aristas", () => {
    const d = doc([
      node("frontera", 100, 100, {}, "package"),
      node("caso", 140, 140, { parent: "frontera" }, "ellipse"),
      node("actor", 20, 140, {}, "actor"),
    ]);

    const byId = new Map(toFlowNodes(d, NONE).map((n) => [n.id, n]));

    expect(byId.get("frontera")!.zIndex).toBe(-1);
    expect(byId.get("caso")!.zIndex).toBeUndefined();
    expect(byId.get("actor")!.zIndex).toBeUndefined();
  });

  it("un paquete vacio se comporta como una caja normal", () => {
    const d = doc([node("vacio", 0, 0, {}, "package")]);

    expect(toFlowNodes(d, NONE)[0]!.zIndex).toBeUndefined();
  });
});

describe("todo nodo llega medido", () => {
  // React Flow considera «sin medir» a todo nodo que no traiga `measured`, y
  // un nodo sin medir no se puede arrastrar: avisa con «trying to drag a node
  // that is not initialized» y se queda clavado.
  //
  // Esto ya se rompio una vez. Al dejar de guardar en el documento lo que
  // React Flow mide del DOM —para cortar el bucle de repintado— nadie
  // rellenaba `measured` y el lienzo se quedo sin arrastre. Mientras estas
  // pruebas pasen, esa combinacion no puede repetirse.

  it("con su geometria, cuando la tiene", () => {
    const [flow] = toFlowNodes(doc([node("a", 40, 80)]), NONE);

    expect(flow!.measured).toEqual({ width: 160, height: 60 });
  });

  it("y con un tamano de reserva cuando todavia no la tiene", () => {
    // Un documento recien importado llega sin geometria: la pone el reparto
    // despues. Hasta entonces el nodo tiene que poder arrastrarse igual.
    const sinGeometria = node("a", 0, 0, { geometry: null });
    const [flow] = toFlowNodes(doc([sinGeometria]), NONE);

    expect(typeof flow!.measured?.width).toBe("number");
    expect(typeof flow!.measured?.height).toBe("number");
  });

  it("todos, sea cual sea su forma", () => {
    const formas: NodeShape[] = [
      "rectangle",
      "roundedRect",
      "ellipse",
      "diamond",
      "cylinder",
      "actor",
      "package",
      "text",
    ];
    const d = doc(formas.map((shape, i) => node(`n${i}`, i * 200, 0, {}, shape)));

    for (const flow of toFlowNodes(d, NONE)) {
      expect(flow.measured?.width, `${flow.type} sin ancho`).toBeGreaterThan(0);
      expect(flow.measured?.height, `${flow.type} sin alto`).toBeGreaterThan(0);
    }
  });

  it("y lo medido coincide con lo que se dibuja", () => {
    // Si `measured` dijera una cosa y `width`/`height` otra, las aristas
    // acabarian apuntando a un borde que no esta donde se ve.
    const d = doc([node("a", 0, 0), node("b", 300, 0, {}, "package")]);

    for (const flow of toFlowNodes(d, NONE)) {
      expect(flow.measured).toEqual({ width: flow.width, height: flow.height });
    }
  });
});
