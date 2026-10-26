import { produce } from "immer";
import { describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { ErColumn } from "@/model/generated/ErColumn";
import type { Node } from "@/model/generated/Node";
import { TABLE_HEADER_HEIGHT, TABLE_ROW_HEIGHT } from "@/model/tableMetrics";
import { __testing, applyAutoLayout, sizeOf } from "./autoLayout";

function column(name: string, type = "integer"): ErColumn {
  return {
    id: `c_${name}`,
    name,
    dataType: type,
    primaryKey: false,
    foreignKey: false,
    nullable: true,
    unique: false,
    increment: false,
  };
}

function tableNode(id: string, columns: ErColumn[] = [], parent?: string): Node {
  const node: Node = {
    id,
    label: id,
    shape: "table",
    pinned: false,
    data: { type: "er", columns, alias: undefined, schema: undefined, note: undefined },
  };
  if (parent) node.parent = parent;
  return node;
}

function doc(nodes: Node[], edges: Document["edges"] = []): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "er",
    meta: { title: "Prueba", created: 0, modified: 0 },
    nodes,
    edges,
    layout: "auto",
    direction: "right",
  };
}

const edge = (
  id: string,
  source: string,
  target: string,
): Document["edges"][number] => ({
  id,
  source: { node: source },
  target: { node: target },
  kind: "association",
  routing: "orthogonal",
});

describe("sizeOf", () => {
  it("una tabla crece una fila por columna", () => {
    const sin = sizeOf(tableNode("a"));
    const con = sizeOf(tableNode("b", [column("id"), column("email")]));

    expect(sin.height).toBe(TABLE_HEADER_HEIGHT);
    expect(con.height).toBe(TABLE_HEADER_HEIGHT + 2 * TABLE_ROW_HEIGHT);
  });

  it("respeta la geometria que ya tiene el nodo", () => {
    const node = tableNode("a", [column("id")]);
    node.geometry = { x: 0, y: 0, width: 333, height: 111 };

    expect(sizeOf(node)).toEqual({ width: 333, height: 111 });
  });

  it("una forma normal usa el tamano por defecto de su forma", () => {
    const node: Node = {
      id: "n",
      label: "n",
      shape: "rectangle",
      pinned: false,
      data: { type: "plain" },
    };

    expect(sizeOf(node)).toEqual({ width: 160, height: 60 });
  });
});

describe("toElkGraph", () => {
  it("cuelga los hijos de su contenedor", () => {
    const graph = __testing.toElkGraph(
      doc([
        { ...tableNode("grupo"), shape: "package", data: { type: "plain" } },
        tableNode("a", [], "grupo"),
      ]),
    );

    expect(graph.children).toHaveLength(1);
    expect(graph.children![0]!.id).toBe("grupo");
    expect(graph.children![0]!.children).toHaveLength(1);
  });

  it("descarta los bucles sobre un mismo nodo", () => {
    // ELK los coloca mal y no aportan nada al orden de las capas.
    const graph = __testing.toElkGraph(
      doc(
        [tableNode("a"), tableNode("b")],
        [edge("e1", "a", "a"), edge("e2", "a", "b")],
      ),
    );

    expect(graph.edges).toHaveLength(1);
    expect(graph.edges![0]!.id).toBe("e2");
  });

  it("fija la posicion de los nodos anclados", () => {
    const anchored = tableNode("a");
    anchored.pinned = true;
    anchored.geometry = { x: 50, y: 70, width: 200, height: 50 };

    const graph = __testing.toElkGraph(doc([anchored]));

    expect(graph.children![0]).toMatchObject({ x: 50, y: 70 });
  });
});

describe("applyAutoLayout", () => {
  it("coloca los nodos que llegan sin geometria", async () => {
    const before = doc([
      tableNode("a", [column("id")]),
      tableNode("b", [column("id")]),
    ]);

    const after = await produceAsync(before, applyAutoLayout);

    for (const node of after.nodes) {
      expect(node.geometry, node.id).toBeDefined();
      expect(Number.isFinite(node.geometry!.x)).toBe(true);
    }
  });

  it("no apila dos tablas en el mismo sitio", async () => {
    const after = await produceAsync(
      doc(
        [tableNode("a", [column("id")]), tableNode("b", [column("id")])],
        [edge("e1", "a", "b")],
      ),
      applyAutoLayout,
    );

    const [a, b] = after.nodes;
    expect(a!.geometry).not.toEqual(b!.geometry);
  });

  it("deja quietos los nodos anclados", async () => {
    const anchored = tableNode("a", [column("id")]);
    anchored.pinned = true;
    anchored.geometry = { x: 500, y: 300, width: 200, height: 50 };

    const after = await produceAsync(doc([anchored, tableNode("b")]), applyAutoLayout);

    expect(after.nodes[0]!.geometry).toMatchObject({ x: 500, y: 300 });
  });

  it("el tamano de cada nodo corresponde a su contenido", async () => {
    const after = await produceAsync(
      doc([tableNode("a", [column("id"), column("email"), column("rol")])]),
      applyAutoLayout,
    );

    expect(after.nodes[0]!.geometry!.height).toBe(
      TABLE_HEADER_HEIGHT + 3 * TABLE_ROW_HEIGHT,
    );
  });

  it("un documento vacio no revienta", async () => {
    const after = await produceAsync(doc([]), applyAutoLayout);

    expect(after.nodes).toHaveLength(0);
  });

  it("las posiciones son enteras: media unidad de diagrama no significa nada", async () => {
    const after = await produceAsync(
      doc([tableNode("a", [column("id")]), tableNode("b")], [edge("e1", "a", "b")]),
      applyAutoLayout,
    );

    for (const node of after.nodes) {
      expect(Number.isInteger(node.geometry!.x)).toBe(true);
      expect(Number.isInteger(node.geometry!.y)).toBe(true);
    }
  });
});

/** `produce` de immer con una receta asincrona. */
async function produceAsync(
  base: Document,
  recipe: (draft: Document) => Promise<void>,
): Promise<Document> {
  const draft = structuredClone(base);
  await recipe(draft);
  return produce(base, () => draft);
}

describe("direccion del grafo", () => {
  it("se la pasa a ELK tal y como la trae el documento", () => {
    for (const [direction, elk] of [
      ["right", "RIGHT"],
      ["left", "LEFT"],
      ["down", "DOWN"],
      ["up", "UP"],
    ] as const) {
      const graph = __testing.toElkGraph({
        ...doc([tableNode("a")]),
        direction,
      });

      expect(graph.layoutOptions!["elk.direction"], direction).toBe(elk);
    }
  });

  it("un diagrama de arriba abajo apila los nodos en vertical", async () => {
    const vertical = await produceAsync(
      {
        ...doc(
          [tableNode("a", [column("id")]), tableNode("b", [column("id")])],
          [edge("e1", "a", "b")],
        ),
        direction: "down",
      },
      applyAutoLayout,
    );

    const [a, b] = vertical.nodes;
    // Con direccion `down`, el segundo cae por debajo, no al lado.
    expect(b!.geometry!.y).toBeGreaterThan(a!.geometry!.y);
  });
});

describe("el tamaño de un contenedor", () => {
  function packageNode(id: string): Node {
    return { id, label: id, shape: "package", pinned: false, data: { type: "plain" } };
  }

  /** `true` si el contenedor encierra a todos sus hijos. */
  function encierra(document: Document, parent: string): boolean {
    const caja = document.nodes.find((n) => n.id === parent)?.geometry;
    if (!caja) return false;

    return document.nodes
      .filter((n) => n.parent === parent)
      .every((hijo) => {
        const g = hijo.geometry;
        return (
          g != null &&
          g.x >= caja.x &&
          g.y >= caja.y &&
          g.x + g.width <= caja.x + caja.width &&
          g.y + g.height <= caja.y + caja.height
        );
      });
  }

  /** Caja de un nodo, ya colocado. */
  function caja(document: Document, id: string) {
    return document.nodes.find((n) => n.id === id)!.geometry!;
  }

  /** Coloca un paquete con tantas tablas dentro como se le pidan. */
  async function conHijos(cuantos: number): Promise<Document> {
    const hijos = Array.from({ length: cuantos }, (_, i) =>
      tableNode(`t${i}`, [column("id"), column("nombre"), column("fecha")], "grupo"),
    );
    return produceAsync(doc([packageNode("grupo"), ...hijos]), applyAutoLayout);
  }

  it("un paquete encierra lo que lleva dentro", async () => {
    // Antes se quedaba clavado en el tamaño por defecto de su forma, asi que
    // con varias tablas dentro el contenido salia fuera de la caja.
    const document = await conHijos(6);

    expect(encierra(document, "grupo")).toBe(true);
  });

  it("y crece con lo que le vayan metiendo", async () => {
    const uno = caja(await conHijos(1), "grupo");
    const seis = caja(await conHijos(6), "grupo");

    expect(seis.width * seis.height).toBeGreaterThan(uno.width * uno.height);
  });

  it("un paquete con un solo hijo se ajusta a el, no al tamaño de su forma", async () => {
    const document = await produceAsync(
      doc([packageNode("grupo"), tableNode("a", [column("id")], "grupo")]),
      applyAutoLayout,
    );
    const grupo = caja(document, "grupo");
    const hijo = caja(document, "a");

    expect(encierra(document, "grupo")).toBe(true);
    // Pegado a su contenido: lo que sobra es el relleno del contenedor, no
    // medio paquete vacio.
    expect(grupo.width - hijo.width).toBeLessThan(80);
  });

  it("los paquetes anidados encierran cada uno lo suyo", async () => {
    const document = await produceAsync(
      doc([
        packageNode("fuera"),
        { ...packageNode("dentro"), parent: "fuera" },
        tableNode("a", [column("id"), column("nombre")], "dentro"),
        tableNode("b", [column("id"), column("nombre")], "dentro"),
        tableNode("c", [column("id")], "fuera"),
      ]),
      applyAutoLayout,
    );

    expect(encierra(document, "dentro")).toBe(true);
    expect(encierra(document, "fuera")).toBe(true);
  });

  it("un nodo suelto conserva el tamaño de su forma", async () => {
    const document = await produceAsync(
      doc([tableNode("a", [column("id")])]),
      applyAutoLayout,
    );

    expect(document.nodes[0]!.geometry!.height).toBe(
      TABLE_HEADER_HEIGHT + TABLE_ROW_HEIGHT,
    );
  });
});
