import { produce } from "immer";
import { describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import {
  addNode,
  connect,
  moveNode,
  newId,
  reconnect,
  removeElements,
  resizeNode,
  setEdgeKind,
  setEndpointLabel,
  setEndpointSide,
  setLabel,
} from "./operations";

function emptyDoc(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "architecture",
    meta: { title: "Prueba", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "auto",
  };
}

/** Aplica una operacion y devuelve el documento resultante. */
function run(doc: Document, recipe: (draft: Document) => void): Document {
  return produce(doc, recipe);
}

describe("newId", () => {
  it("usa el prefijo pedido y no se repite", () => {
    const a = newId("n");
    const b = newId("n");

    expect(a.startsWith("n_")).toBe(true);
    expect(a).not.toBe(b);
  });
});

describe("addNode", () => {
  it("anade un nodo con geometria y anclado", () => {
    const doc = run(emptyDoc(), (d) => {
      addNode(d, { shape: "rectangle", label: "API", x: 10, y: 20 });
    });

    expect(doc.nodes).toHaveLength(1);
    const node = doc.nodes[0]!;
    expect(node.label).toBe("API");
    expect(node.geometry).toEqual({ x: 10, y: 20, width: 160, height: 60 });
    expect(node.pinned).toBe(true);
  });

  it("toma el tamano por defecto de cada forma", () => {
    const doc = run(emptyDoc(), (d) => {
      addNode(d, { shape: "cylinder", x: 0, y: 0 });
      addNode(d, { shape: "actor", x: 0, y: 0 });
    });

    expect(doc.nodes[0]!.geometry).toMatchObject({ width: 120, height: 90 });
    expect(doc.nodes[1]!.geometry).toMatchObject({ width: 40, height: 80 });
  });

  it("registra el contenedor cuando se indica", () => {
    const doc = run(emptyDoc(), (d) => {
      const pkg = addNode(d, { shape: "package", x: 0, y: 0 });
      addNode(d, { shape: "rectangle", x: 10, y: 10, parent: pkg });
    });

    expect(doc.nodes[1]!.parent).toBe(doc.nodes[0]!.id);
  });
});

describe("connect", () => {
  it("crea la arista entre dos nodos existentes", () => {
    let source = "";
    let target = "";
    const doc = run(emptyDoc(), (d) => {
      source = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      target = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      connect(d, source, target, "flow");
    });

    expect(doc.edges).toHaveLength(1);
    expect(doc.edges[0]).toMatchObject({
      source: { node: source },
      target: { node: target },
      kind: "flow",
      routing: "orthogonal",
    });
  });

  it("guarda el lado desde el que arrastro el usuario", () => {
    const doc = run(emptyDoc(), (d) => {
      const a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      const b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      connect(d, a, b, "association", { source: "bottom", target: "top" });
    });

    expect(doc.edges[0]!.source.side).toBe("bottom");
    expect(doc.edges[0]!.target.side).toBe("top");
  });

  it("sin lado elegido no fija ninguno y decide el lienzo", () => {
    const doc = run(emptyDoc(), (d) => {
      const a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      const b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      connect(d, a, b);
    });

    expect(doc.edges[0]!.source.side).toBeUndefined();
    expect(doc.edges[0]!.target.side).toBeUndefined();
  });

  it("rechaza conectar con un nodo inexistente", () => {
    let result: string | null = "no ejecutado";
    const doc = run(emptyDoc(), (d) => {
      const a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      result = connect(d, a, "fantasma");
    });

    expect(result).toBeNull();
    expect(doc.edges).toHaveLength(0);
  });
});

describe("moveNode / resizeNode", () => {
  it("mover ancla el nodo", () => {
    const base = run(emptyDoc(), (d) => {
      addNode(d, { shape: "rectangle", x: 0, y: 0 });
    });
    const id = base.nodes[0]!.id;

    const doc = run(base, (d) => moveNode(d, id, 50, 80));

    expect(doc.nodes[0]!.geometry).toMatchObject({ x: 50, y: 80 });
    expect(doc.nodes[0]!.pinned).toBe(true);
  });

  it("redimensionar conserva la posicion", () => {
    const base = run(emptyDoc(), (d) => {
      addNode(d, { shape: "rectangle", x: 7, y: 9 });
    });
    const id = base.nodes[0]!.id;

    const doc = run(base, (d) => resizeNode(d, id, 300, 120));

    expect(doc.nodes[0]!.geometry).toEqual({ x: 7, y: 9, width: 300, height: 120 });
  });

  it("mover un id inexistente no rompe nada", () => {
    const base = emptyDoc();
    const doc = run(base, (d) => moveNode(d, "fantasma", 1, 1));

    expect(doc).toEqual(base);
  });
});

describe("setLabel", () => {
  it("renombra nodos y aristas con la misma funcion", () => {
    let nodeId = "";
    let edgeId: string | null = "";
    const base = run(emptyDoc(), (d) => {
      nodeId = addNode(d, { shape: "rectangle", x: 0, y: 0, label: "viejo" });
      const other = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      edgeId = connect(d, nodeId, other);
    });

    const doc = run(base, (d) => {
      setLabel(d, nodeId, "nuevo");
      setLabel(d, edgeId!, "usa");
    });

    expect(doc.nodes[0]!.label).toBe("nuevo");
    expect(doc.edges[0]!.label).toBe("usa");
  });

  it("una etiqueta vacia borra la de la arista en vez de dejarla en blanco", () => {
    let edgeId: string | null = "";
    const base = run(emptyDoc(), (d) => {
      const a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      const b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      edgeId = connect(d, a, b);
      setLabel(d, edgeId!, "algo");
    });

    const doc = run(base, (d) => setLabel(d, edgeId!, ""));

    expect(doc.edges[0]!.label).toBeUndefined();
  });
});

describe("removeElements", () => {
  it("borra el nodo y las aristas que lo tocan", () => {
    let a = "";
    const base = run(emptyDoc(), (d) => {
      a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      const b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      const c = addNode(d, { shape: "rectangle", x: 600, y: 0 });
      connect(d, a, b);
      connect(d, b, c);
    });

    const doc = run(base, (d) => removeElements(d, [a]));

    expect(doc.nodes.map((n) => n.id)).not.toContain(a);
    expect(doc.nodes).toHaveLength(2);
    expect(doc.edges).toHaveLength(1);
  });

  it("borrar un contenedor arrastra a sus hijos", () => {
    let pkg = "";
    const base = run(emptyDoc(), (d) => {
      pkg = addNode(d, { shape: "package", x: 0, y: 0 });
      addNode(d, { shape: "rectangle", x: 10, y: 10, parent: pkg });
      addNode(d, { shape: "rectangle", x: 500, y: 0 });
    });

    const doc = run(base, (d) => removeElements(d, [pkg]));

    expect(doc.nodes).toHaveLength(1);
  });

  it("arrastra tambien a los nietos", () => {
    let raiz = "";
    const base = run(emptyDoc(), (d) => {
      raiz = addNode(d, { shape: "package", x: 0, y: 0 });
      const medio = addNode(d, { shape: "package", x: 10, y: 10, parent: raiz });
      addNode(d, { shape: "rectangle", x: 20, y: 20, parent: medio });
    });

    const doc = run(base, (d) => removeElements(d, [raiz]));

    expect(doc.nodes).toHaveLength(0);
  });

  it("borra una arista sin tocar sus nodos", () => {
    let edgeId: string | null = "";
    const base = run(emptyDoc(), (d) => {
      const a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      const b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      edgeId = connect(d, a, b);
    });

    const doc = run(base, (d) => removeElements(d, [edgeId!]));

    expect(doc.nodes).toHaveLength(2);
    expect(doc.edges).toHaveLength(0);
  });
});

describe("reconnect", () => {
  function base() {
    const ids = { a: "", b: "", c: "", e: null as string | null };
    const doc = run(emptyDoc(), (d) => {
      ids.a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      ids.b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      ids.c = addNode(d, { shape: "rectangle", x: 600, y: 0 });
      ids.e = connect(d, ids.a, ids.b, "flow", { source: "right", target: "left" });
    });
    return { doc, ids };
  }

  it("mueve un extremo a otro nodo conservando tipo y etiqueta", () => {
    const { doc: before, ids } = base();
    const conEtiqueta = run(before, (d) => setLabel(d, ids.e!, "usa"));

    const doc = run(conEtiqueta, (d) => reconnect(d, ids.e!, "target", ids.c, "top"));

    expect(doc.edges[0]!.target.node).toBe(ids.c);
    expect(doc.edges[0]!.target.side).toBe("top");
    expect(doc.edges[0]!.kind).toBe("flow");
    expect(doc.edges[0]!.label).toBe("usa");
  });

  it("reenganchar sin lado vuelve el extremo a automatico", () => {
    const { doc: before, ids } = base();

    const doc = run(before, (d) => reconnect(d, ids.e!, "source", ids.a));

    expect(doc.edges[0]!.source.side).toBeUndefined();
  });

  it("ignora un nodo destino inexistente", () => {
    const { doc: before, ids } = base();

    const doc = run(before, (d) => reconnect(d, ids.e!, "target", "fantasma", "top"));

    expect(doc.edges[0]!.target.node).toBe(ids.b);
  });

  it("setEndpointSide libera el lado al pasar undefined", () => {
    const { doc: before, ids } = base();

    const doc = run(before, (d) => setEndpointSide(d, ids.e!, "source", undefined));

    expect(doc.edges[0]!.source.side).toBeUndefined();
  });
});

describe("aristas", () => {
  it("guarda las cardinalidades en cada extremo", () => {
    let edgeId: string | null = "";
    const base = run(emptyDoc(), (d) => {
      const a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      const b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      edgeId = connect(d, a, b);
    });

    const doc = run(base, (d) => {
      setEndpointLabel(d, edgeId!, "source", "1");
      setEndpointLabel(d, edgeId!, "target", "0..*");
      setEdgeKind(d, edgeId!, "composition");
    });

    expect(doc.edges[0]!.source.label).toBe("1");
    expect(doc.edges[0]!.target.label).toBe("0..*");
    expect(doc.edges[0]!.kind).toBe("composition");
  });
});
