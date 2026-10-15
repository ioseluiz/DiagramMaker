import { produce } from "immer";
import { describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import { TABLE_ROW_HEIGHT } from "@/model/tableMetrics";
import {
  addColumn,
  addNode,
  addTable,
  connect,
  duplicateNode,
  groupNodes,
  moveColumn,
  moveNode,
  newId,
  reconnect,
  relationKindOf,
  removeColumn,
  removeElements,
  resizeNode,
  reverseEdge,
  setEdgeKind,
  setEndpointLabel,
  setEndpointSide,
  setLabel,
  setNodeParent,
  setRelationKind,
  setTableAlias,
  setTableNote,
  ungroupNode,
  updateColumn,
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

describe("tablas entidad-relacion", () => {
  function erDoc(): Document {
    return { ...emptyDoc(), kind: "er" };
  }

  function tableOf(doc: Document, id: string) {
    const node = doc.nodes.find((n) => n.id === id)!;
    if (node.data.type !== "er") throw new Error("no es una tabla");
    return node.data;
  }

  it("una tabla nueva nace con una clave primaria `id`", () => {
    let id = "";
    const doc = run(erDoc(), (d) => {
      id = addTable(d, { x: 10, y: 20 });
    });

    const table = tableOf(doc, id);
    expect(table.columns).toHaveLength(1);
    expect(table.columns[0]!.name).toBe("id");
    expect(table.columns[0]!.primaryKey).toBe(true);
    expect(table.columns[0]!.nullable).toBe(false);
  });

  it("la tabla nueva queda colocada y anclada", () => {
    let id = "";
    const doc = run(erDoc(), (d) => {
      id = addTable(d, { x: 10, y: 20 });
    });

    const node = doc.nodes.find((n) => n.id === id)!;
    expect(node.geometry).toMatchObject({ x: 10, y: 20 });
    expect(node.pinned).toBe(true);
    expect(node.shape).toBe("table");
  });

  it("anadir una columna sube el alto una fila", () => {
    let id = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
    });
    const before = base.nodes[0]!.geometry!.height;

    const doc = run(base, (d) => {
      addColumn(d, id);
    });

    expect(doc.nodes[0]!.geometry!.height).toBe(before + TABLE_ROW_HEIGHT);
  });

  it("las columnas nuevas no repiten nombre", () => {
    let id = "";
    const doc = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
      addColumn(d, id);
      addColumn(d, id);
      addColumn(d, id);
    });

    const names = tableOf(doc, id).columns.map((c) => c.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("borrar una columna baja el alto", () => {
    let id = "";
    let columnId: string | null = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
      columnId = addColumn(d, id);
    });
    const before = base.nodes[0]!.geometry!.height;

    const doc = run(base, (d) => removeColumn(d, id, columnId!));

    expect(doc.nodes[0]!.geometry!.height).toBe(before - TABLE_ROW_HEIGHT);
    expect(tableOf(doc, id).columns).toHaveLength(1);
  });

  it("borrar una columna conserva la relacion y solo suelta el anclaje", () => {
    let a = "";
    let b = "";
    let columnId: string | null = "";
    let edgeId: string | null = "";

    const base = run(erDoc(), (d) => {
      a = addTable(d, { x: 0, y: 0 });
      b = addTable(d, { x: 400, y: 0 });
      columnId = addColumn(d, a);
      edgeId = connect(d, a, b);
      const edge = d.edges.find((e) => e.id === edgeId)!;
      edge.source.port = columnId!;
    });

    const doc = run(base, (d) => removeColumn(d, a, columnId!));

    expect(doc.edges).toHaveLength(1);
    expect(doc.edges[0]!.source.port).toBeUndefined();
    expect(doc.edges[0]!.source.node).toBe(a);
  });

  it("marcar una columna como clave primaria le quita la nulabilidad", () => {
    let id = "";
    let columnId: string | null = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
      columnId = addColumn(d, id);
    });

    const doc = run(base, (d) =>
      updateColumn(d, id, columnId!, { primaryKey: true, nullable: true }),
    );

    const column = tableOf(doc, id).columns[1]!;
    expect(column.primaryKey).toBe(true);
    expect(column.nullable).toBe(false);
  });

  it("cambiar el tipo y el nombre de una columna", () => {
    let id = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
    });
    const columnId = tableOf(base, id).columns[0]!.id;

    const doc = run(base, (d) =>
      updateColumn(d, id, columnId, { name: "clave", dataType: "bigint" }),
    );

    expect(tableOf(doc, id).columns[0]).toMatchObject({
      name: "clave",
      dataType: "bigint",
    });
  });

  it("la tabla se ensancha si una columna no cabe, pero nunca encoge", () => {
    let id = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
    });
    const columnId = tableOf(base, id).columns[0]!.id;
    const ancha = run(base, (d) => {
      d.nodes[0]!.geometry!.width = 400;
      updateColumn(d, id, columnId, { name: "x" });
    });

    expect(ancha.nodes[0]!.geometry!.width).toBe(400);
  });

  it("reordenar columnas con los dos sentidos", () => {
    let id = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
      addColumn(d, id, "segunda");
      addColumn(d, id, "tercera");
    });

    const bajada = run(base, (d) =>
      moveColumn(d, id, tableOf(base, id).columns[0]!.id, 1),
    );
    expect(tableOf(bajada, id).columns.map((c) => c.name)).toEqual([
      "segunda",
      "id",
      "tercera",
    ]);

    const subida = run(base, (d) =>
      moveColumn(d, id, tableOf(base, id).columns[2]!.id, -1),
    );
    expect(tableOf(subida, id).columns.map((c) => c.name)).toEqual([
      "id",
      "tercera",
      "segunda",
    ]);
  });

  it("no se puede sacar una columna de los extremos", () => {
    let id = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
      addColumn(d, id, "segunda");
    });
    const first = tableOf(base, id).columns[0]!.id;
    const last = tableOf(base, id).columns[1]!.id;

    expect(
      tableOf(
        run(base, (d) => moveColumn(d, id, first, -1)),
        id,
      ).columns[0]!.name,
    ).toBe("id");
    expect(
      tableOf(
        run(base, (d) => moveColumn(d, id, last, 1)),
        id,
      ).columns[1]!.name,
    ).toBe("segunda");
  });

  it("alias y nota se vacian a undefined, no a cadena vacia", () => {
    let id = "";
    const base = run(erDoc(), (d) => {
      id = addTable(d, { x: 0, y: 0 });
      setTableAlias(d, id, "U");
      setTableNote(d, id, "Clientes");
    });

    expect(tableOf(base, id).alias).toBe("U");

    const vacio = run(base, (d) => {
      setTableAlias(d, id, "   ");
      setTableNote(d, id, "");
    });

    expect(tableOf(vacio, id).alias).toBeUndefined();
    expect(tableOf(vacio, id).note).toBeUndefined();
  });

  it("las operaciones de columna ignoran los nodos que no son tablas", () => {
    const base = run(emptyDoc(), (d) => {
      addNode(d, { shape: "rectangle", x: 0, y: 0 });
    });
    const id = base.nodes[0]!.id;

    const doc = run(base, (d) => {
      addColumn(d, id);
      removeColumn(d, id, "nada");
      updateColumn(d, id, "nada", { name: "x" });
      moveColumn(d, id, "nada", 1);
    });

    expect(doc.nodes[0]!.data.type).toBe("plain");
  });
});

describe("relaciones entre columnas", () => {
  function erDoc(): Document {
    return { ...emptyDoc(), kind: "er" };
  }

  function twoTables() {
    const ids = { a: "", b: "", ca: "", cb: "" };
    const doc = run(erDoc(), (d) => {
      ids.a = addTable(d, { x: 0, y: 0, label: "posts" });
      ids.b = addTable(d, { x: 400, y: 0, label: "users" });
      ids.ca = addColumn(d, ids.a, "user_id")!;
      ids.cb = (
        d.nodes.find((n) => n.id === ids.b)!.data as { columns: { id: string }[] }
      ).columns[0]!.id;
    });
    return { doc, ids };
  }

  function columnOf(doc: Document, nodeId: string, columnId: string) {
    const node = doc.nodes.find((n) => n.id === nodeId)!;
    if (node.data.type !== "er") throw new Error("no es una tabla");
    return node.data.columns.find((c) => c.id === columnId)!;
  }

  it("unir dos columnas guarda ambos extremos", () => {
    const { doc: base, ids } = twoTables();

    const doc = run(base, (d) => {
      connect(d, ids.a, ids.b, "association", {}, { source: ids.ca, target: ids.cb });
    });

    expect(doc.edges[0]!.source.port).toBe(ids.ca);
    expect(doc.edges[0]!.target.port).toBe(ids.cb);
  });

  it("una relacion entre columnas nace como muchos a uno", () => {
    const { doc: base, ids } = twoTables();

    const doc = run(base, (d) => {
      connect(d, ids.a, ids.b, "association", {}, { source: ids.ca, target: ids.cb });
    });

    expect(doc.edges[0]!.source.label).toBe("*");
    expect(doc.edges[0]!.target.label).toBe("1");
  });

  it("el lado muchos queda marcado como clave ajena", () => {
    const { doc: base, ids } = twoTables();

    const doc = run(base, (d) => {
      connect(d, ids.a, ids.b, "association", {}, { source: ids.ca, target: ids.cb });
    });

    expect(columnOf(doc, ids.a, ids.ca).foreignKey).toBe(true);
    expect(columnOf(doc, ids.b, ids.cb).foreignKey).toBe(false);
  });

  it("cambiar la cardinalidad mueve la clave ajena al otro lado", () => {
    const { doc: base, ids } = twoTables();
    let edgeId: string | null = "";
    const conectado = run(base, (d) => {
      edgeId = connect(
        d,
        ids.a,
        ids.b,
        "association",
        {},
        { source: ids.ca, target: ids.cb },
      );
    });

    const doc = run(conectado, (d) => setRelationKind(d, edgeId!, "oneToMany"));

    expect(doc.edges[0]!.source.label).toBe("1");
    expect(doc.edges[0]!.target.label).toBe("*");
    expect(columnOf(doc, ids.a, ids.ca).foreignKey).toBe(false);
    expect(columnOf(doc, ids.b, ids.cb).foreignKey).toBe(true);
  });

  it("uno a uno y muchos a muchos no marcan ninguna clave ajena", () => {
    const { doc: base, ids } = twoTables();
    let edgeId: string | null = "";
    const conectado = run(base, (d) => {
      edgeId = connect(
        d,
        ids.a,
        ids.b,
        "association",
        {},
        { source: ids.ca, target: ids.cb },
      );
    });

    for (const kind of ["oneToOne", "manyToMany"] as const) {
      const doc = run(conectado, (d) => setRelationKind(d, edgeId!, kind));

      expect(columnOf(doc, ids.a, ids.ca).foreignKey, kind).toBe(false);
      expect(columnOf(doc, ids.b, ids.cb).foreignKey, kind).toBe(false);
    }
  });

  it("relationKindOf reconoce las cuatro de vuelta", () => {
    const { doc: base, ids } = twoTables();
    let edgeId: string | null = "";
    const conectado = run(base, (d) => {
      edgeId = connect(
        d,
        ids.a,
        ids.b,
        "association",
        {},
        { source: ids.ca, target: ids.cb },
      );
    });

    for (const kind of ["manyToOne", "oneToMany", "oneToOne", "manyToMany"] as const) {
      const doc = run(conectado, (d) => setRelationKind(d, edgeId!, kind));
      expect(relationKindOf(doc.edges[0]!)).toBe(kind);
    }
  });

  it("fuera de un diagrama ER no se inventa cardinalidad", () => {
    let a = "";
    let b = "";
    const doc = run(emptyDoc(), (d) => {
      a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      connect(d, a, b);
    });

    expect(doc.edges[0]!.source.label).toBeUndefined();
    expect(doc.edges[0]!.target.label).toBeUndefined();
  });

  it("reenganchar a otra columna mueve el anclaje", () => {
    const { doc: base, ids } = twoTables();
    let edgeId: string | null = "";
    let otra: string | null = "";
    const conectado = run(base, (d) => {
      otra = addColumn(d, ids.b, "alias");
      edgeId = connect(
        d,
        ids.a,
        ids.b,
        "association",
        {},
        { source: ids.ca, target: ids.cb },
      );
    });

    const doc = run(conectado, (d) =>
      reconnect(d, edgeId!, "target", ids.b, "left", otra!),
    );

    expect(doc.edges[0]!.target.port).toBe(otra);
  });
});

describe("duplicateNode", () => {
  it("copia el nodo desplazado y con id nuevo", () => {
    let id = "";
    const base = run(emptyDoc(), (d) => {
      id = addNode(d, { shape: "rectangle", x: 100, y: 50, label: "API" });
    });

    const doc = run(base, (d) => void duplicateNode(d, id));

    expect(doc.nodes).toHaveLength(2);
    const [original, copia] = doc.nodes;
    expect(copia!.id).not.toBe(original!.id);
    expect(copia!.label).toBe("API");
    expect(copia!.geometry).toMatchObject({ x: 124, y: 74 });
  });

  it("arrastra a los descendientes", () => {
    let pkg = "";
    const base = run(emptyDoc(), (d) => {
      pkg = addNode(d, { shape: "package", x: 0, y: 0 });
      const medio = addNode(d, { shape: "package", x: 10, y: 10, parent: pkg });
      addNode(d, { shape: "rectangle", x: 20, y: 20, parent: medio });
    });

    const doc = run(base, (d) => void duplicateNode(d, pkg));

    expect(doc.nodes).toHaveLength(6);
  });

  it("la copia de un contenedor conserva su propia jerarquia", () => {
    let pkg = "";
    const base = run(emptyDoc(), (d) => {
      pkg = addNode(d, { shape: "package", x: 0, y: 0 });
      addNode(d, { shape: "rectangle", x: 10, y: 10, parent: pkg });
    });

    let copiaId = "";
    const doc = run(base, (d) => {
      copiaId = duplicateNode(d, pkg)!;
    });

    const hijos = doc.nodes.filter((n) => n.parent === copiaId);
    expect(hijos).toHaveLength(1);
    // Y el original no gano hijos de prestado.
    expect(doc.nodes.filter((n) => n.parent === pkg)).toHaveLength(1);
  });

  it("las columnas de una tabla copiada reciben ids nuevos", () => {
    let id = "";
    const base = run({ ...emptyDoc(), kind: "er" }, (d) => {
      id = addTable(d, { x: 0, y: 0 });
      addColumn(d, id, "email");
    });

    const doc = run(base, (d) => void duplicateNode(d, id));

    const todas = doc.nodes.flatMap((n) =>
      n.data.type === "er" ? n.data.columns.map((c) => c.id) : [],
    );
    expect(new Set(todas).size).toBe(todas.length);
  });

  it("la copia no hereda las claves ajenas: sus relaciones no se copian", () => {
    let a = "";
    let b = "";
    const base = run({ ...emptyDoc(), kind: "er" }, (d) => {
      a = addTable(d, { x: 0, y: 0 });
      b = addTable(d, { x: 400, y: 0 });
      const ca = addColumn(d, a, "user_id")!;
      const cb = (
        d.nodes.find((n) => n.id === b)!.data as { columns: { id: string }[] }
      ).columns[0]!.id;
      connect(d, a, b, "association", {}, { source: ca, target: cb });
    });

    const doc = run(base, (d) => void duplicateNode(d, a));

    const copia = doc.nodes.at(-1)!;
    expect(copia.data.type).toBe("er");
    if (copia.data.type !== "er") throw new Error("imposible");
    expect(copia.data.columns.some((c) => c.foreignKey)).toBe(false);
    // Las relaciones no se duplican.
    expect(doc.edges).toHaveLength(1);
  });

  it("duplicar un id inexistente no hace nada", () => {
    const base = emptyDoc();

    expect(run(base, (d) => void duplicateNode(d, "fantasma"))).toEqual(base);
  });
});

describe("reverseEdge", () => {
  it("intercambia los extremos enteros", () => {
    let a = "";
    let b = "";
    let edgeId: string | null = "";
    const base = run({ ...emptyDoc(), kind: "er" }, (d) => {
      a = addTable(d, { x: 0, y: 0 });
      b = addTable(d, { x: 400, y: 0 });
      const ca = addColumn(d, a, "user_id")!;
      const cb = (
        d.nodes.find((n) => n.id === b)!.data as { columns: { id: string }[] }
      ).columns[0]!.id;
      edgeId = connect(d, a, b, "association", {}, { source: ca, target: cb });
    });

    const antes = base.edges[0]!;
    const doc = run(base, (d) => reverseEdge(d, edgeId!));
    const despues = doc.edges[0]!;

    expect(despues.source.node).toBe(antes.target.node);
    expect(despues.target.node).toBe(antes.source.node);
    expect(despues.source.port).toBe(antes.target.port);
    expect(despues.target.port).toBe(antes.source.port);
  });

  it("la cardinalidad viaja con su extremo", () => {
    let a = "";
    let b = "";
    let edgeId: string | null = "";
    const base = run({ ...emptyDoc(), kind: "er" }, (d) => {
      a = addTable(d, { x: 0, y: 0 });
      b = addTable(d, { x: 400, y: 0 });
      const ca = addColumn(d, a, "user_id")!;
      const cb = (
        d.nodes.find((n) => n.id === b)!.data as { columns: { id: string }[] }
      ).columns[0]!.id;
      edgeId = connect(d, a, b, "association", {}, { source: ca, target: cb });
    });

    // Nace como muchos a uno; al invertirla debe leerse uno a muchos.
    const doc = run(base, (d) => reverseEdge(d, edgeId!));

    expect(doc.edges[0]!.source.label).toBe("1");
    expect(doc.edges[0]!.target.label).toBe("*");
    expect(relationKindOf(doc.edges[0]!)).toBe("oneToMany");
  });

  it("invertir dos veces deja la relacion como estaba", () => {
    let a = "";
    let b = "";
    let edgeId: string | null = "";
    const base = run(emptyDoc(), (d) => {
      a = addNode(d, { shape: "rectangle", x: 0, y: 0 });
      b = addNode(d, { shape: "rectangle", x: 300, y: 0 });
      edgeId = connect(d, a, b);
    });

    const doc = run(base, (d) => {
      reverseEdge(d, edgeId!);
      reverseEdge(d, edgeId!);
    });

    expect(doc.edges[0]).toEqual(base.edges[0]);
  });
});

describe("agrupar y desagrupar", () => {
  function twoNodes() {
    const ids = { a: "", b: "" };
    const doc = run(emptyDoc(), (d) => {
      ids.a = addNode(d, { shape: "rectangle", x: 100, y: 100 });
      ids.b = addNode(d, { shape: "rectangle", x: 400, y: 300 });
    });
    return { doc, ids };
  }

  it("el grupo envuelve a sus miembros con margen", () => {
    const { doc: base, ids } = twoNodes();

    let groupId: string | null = "";
    const doc = run(base, (d) => {
      groupId = groupNodes(d, [ids.a, ids.b]);
    });

    const group = doc.nodes.find((n) => n.id === groupId)!;
    const g = group.geometry!;

    // Miembros: de (100,100) a (560,360). El grupo tiene que pasarse por fuera.
    expect(g.x).toBeLessThan(100);
    expect(g.y).toBeLessThan(100);
    expect(g.x + g.width).toBeGreaterThan(560);
    expect(g.y + g.height).toBeGreaterThan(360);
  });

  it("deja sitio arriba para la solapa con el nombre", () => {
    const { doc: base, ids } = twoNodes();

    let groupId: string | null = "";
    const doc = run(base, (d) => {
      groupId = groupNodes(d, [ids.a, ids.b]);
    });

    const g = doc.nodes.find((n) => n.id === groupId)!.geometry!;
    // Por arriba se reserva mas hueco que por los lados.
    expect(100 - g.y).toBeGreaterThan(g.x === 0 ? 0 : 100 - g.x);
  });

  it("el contenedor aparece antes que sus hijos", () => {
    const { doc: base, ids } = twoNodes();

    const doc = run(base, (d) => void groupNodes(d, [ids.a, ids.b]));

    expect(doc.nodes[0]!.shape).toBe("package");
  });

  it("los miembros quedan colgando del grupo", () => {
    const { doc: base, ids } = twoNodes();

    let groupId: string | null = "";
    const doc = run(base, (d) => {
      groupId = groupNodes(d, [ids.a, ids.b]);
    });

    expect(doc.nodes.filter((n) => n.parent === groupId)).toHaveLength(2);
  });

  it("no se lleva nodos que ya viven en otro grupo", () => {
    let pkg = "";
    let dentro = "";
    let fuera = "";
    const base = run(emptyDoc(), (d) => {
      pkg = addNode(d, { shape: "package", x: 0, y: 0 });
      dentro = addNode(d, { shape: "rectangle", x: 20, y: 20, parent: pkg });
      fuera = addNode(d, { shape: "rectangle", x: 500, y: 20 });
    });

    const doc = run(base, (d) => void groupNodes(d, [dentro, fuera]));

    // `dentro` sigue en su paquete original.
    expect(doc.nodes.find((n) => n.id === dentro)!.parent).toBe(pkg);
  });

  it("agrupar sin nada seleccionado no crea nada", () => {
    const base = emptyDoc();

    expect(run(base, (d) => void groupNodes(d, []))).toEqual(base);
  });

  it("desagrupar sube a los hijos y borra el contenedor", () => {
    const { doc: base, ids } = twoNodes();
    let groupId: string | null = "";
    const agrupado = run(base, (d) => {
      groupId = groupNodes(d, [ids.a, ids.b]);
    });

    const doc = run(agrupado, (d) => ungroupNode(d, groupId!));

    expect(doc.nodes).toHaveLength(2);
    expect(doc.nodes.every((n) => !n.parent)).toBe(true);
  });

  it("desagrupar un grupo anidado sube a los hijos un solo nivel", () => {
    let externo = "";
    let interno = "";
    let hoja = "";
    const base = run(emptyDoc(), (d) => {
      externo = addNode(d, { shape: "package", x: 0, y: 0 });
      interno = addNode(d, { shape: "package", x: 10, y: 10, parent: externo });
      hoja = addNode(d, { shape: "rectangle", x: 20, y: 20, parent: interno });
    });

    const doc = run(base, (d) => ungroupNode(d, interno));

    expect(doc.nodes.find((n) => n.id === hoja)!.parent).toBe(externo);
  });

  it("el documento resultante sigue siendo coherente", () => {
    const { doc: base, ids } = twoNodes();

    const doc = run(base, (d) => void groupNodes(d, [ids.a, ids.b]));

    // Ningun nodo apunta a un contenedor inexistente.
    for (const node of doc.nodes) {
      if (node.parent) {
        expect(doc.nodes.some((n) => n.id === node.parent)).toBe(true);
      }
    }
  });
});

describe("setNodeParent", () => {
  function nested() {
    const ids = { pkg: "", hijo: "", suelto: "" };
    const doc = run(emptyDoc(), (d) => {
      ids.pkg = addNode(d, { shape: "package", x: 0, y: 0 });
      ids.hijo = addNode(d, { shape: "rectangle", x: 20, y: 20, parent: ids.pkg });
      ids.suelto = addNode(d, { shape: "rectangle", x: 500, y: 0 });
    });
    return { doc, ids };
  }

  it("mete un nodo suelto en un contenedor", () => {
    const { doc: base, ids } = nested();

    const doc = run(base, (d) => setNodeParent(d, ids.suelto, ids.pkg));

    expect(doc.nodes.find((n) => n.id === ids.suelto)!.parent).toBe(ids.pkg);
  });

  it("null lo devuelve a la raiz", () => {
    const { doc: base, ids } = nested();

    const doc = run(base, (d) => setNodeParent(d, ids.hijo, null));

    expect(doc.nodes.find((n) => n.id === ids.hijo)!.parent).toBeUndefined();
  });

  it("rechaza meter un nodo dentro de si mismo", () => {
    const { doc: base, ids } = nested();

    const doc = run(base, (d) => setNodeParent(d, ids.pkg, ids.pkg));

    expect(doc.nodes.find((n) => n.id === ids.pkg)!.parent).toBeUndefined();
  });

  it("rechaza meter un contenedor dentro de su propio hijo", () => {
    // Seria un ciclo de anidamiento: `validate` lo marca como error y el
    // auto-layout jerarquico se colgaria.
    const { doc: base, ids } = nested();

    const doc = run(base, (d) => setNodeParent(d, ids.pkg, ids.hijo));

    expect(doc.nodes.find((n) => n.id === ids.pkg)!.parent).toBeUndefined();
  });

  it("rechaza un contenedor inexistente", () => {
    const { doc: base, ids } = nested();

    const doc = run(base, (d) => setNodeParent(d, ids.suelto, "fantasma"));

    expect(doc.nodes.find((n) => n.id === ids.suelto)!.parent).toBeUndefined();
  });
});

describe("tipo por defecto de una columna nueva", () => {
  it("una columna nueva trae tipo", () => {
    // Sin tipo, el DBML generado es `columna` a secas y dbdiagram.io lo
    // rechaza: alli el tipo es obligatorio.
    let id = "";
    const doc = run({ ...emptyDoc(), kind: "er" }, (d) => {
      id = addTable(d, { x: 0, y: 0 });
      addColumn(d, id);
    });

    const node = doc.nodes.find((n) => n.id === id)!;
    if (node.data.type !== "er") throw new Error("no es una tabla");

    expect(node.data.columns[1]!.dataType).not.toBe("");
  });
});
