import { produce } from "immer";
import { describe, expect, it } from "vitest";

import type { ClassBox } from "@/model/generated/ClassBox";
import type { Document } from "@/model/generated/Document";
import { classHeight } from "@/model/classMetrics";
import {
  addClass,
  addClassMember,
  moveClassMember,
  removeClassMember,
  setClassStereotype,
  updateClassMember,
} from "./operations";

function emptyClassDiagram(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "class",
    meta: { title: "Prueba", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "manual",
    direction: "right",
  };
}

function run(doc: Document, fn: (draft: Document) => void): Document {
  return produce(doc, (draft) => {
    fn(draft);
  });
}

/** Documento con una clase y su id. */
function conClase(): { doc: Document; id: string } {
  let id = "";
  const doc = run(emptyClassDiagram(), (d) => {
    id = addClass(d, { x: 0, y: 0 });
  });
  return { doc, id };
}

function boxOf(doc: Document, id: string): ClassBox {
  const node = doc.nodes.find((n) => n.id === id)!;
  if (node.data.type !== "class") throw new Error("no es una clase");
  return node.data;
}

describe("crear una clase", () => {
  it("nace con un atributo y una operacion", () => {
    // Una caja con los tres compartimentos vacios no deja ver que es una
    // clase, y lo primero que hace cualquiera es anadir un miembro.
    const { doc, id } = conClase();
    const box = boxOf(doc, id);

    expect(box.attributes).toHaveLength(1);
    expect(box.operations).toHaveLength(1);
  });

  it("su alto sale de sus miembros", () => {
    const { doc, id } = conClase();
    const node = doc.nodes.find((n) => n.id === id)!;

    expect(node.geometry?.height).toBe(classHeight(false, 1, 1));
  });
});

describe("miembros", () => {
  it("anadir uno lo deja al final de su compartimento", () => {
    const { doc: base, id } = conClase();

    const doc = run(base, (d) => {
      addClassMember(d, id, "attributes");
    });

    expect(boxOf(doc, id).attributes).toHaveLength(2);
    expect(boxOf(doc, id).operations).toHaveLength(1);
  });

  it("el alto crece con cada miembro", () => {
    const { doc: base, id } = conClase();
    const antes = base.nodes[0]!.geometry!.height;

    const doc = run(base, (d) => {
      addClassMember(d, id, "operations");
    });

    expect(doc.nodes[0]!.geometry!.height).toBeGreaterThan(antes);
  });

  it("el ancho no encoge al borrar el miembro mas largo", () => {
    // Encogerla seria pelearse con quien la habia ensanchado a mano.
    const { doc: base, id } = conClase();
    const ancho = base.nodes[0]!.geometry!.width;

    const doc = run(base, (d) => {
      const memberId = boxOf(d, id).attributes[0]!.id;
      removeClassMember(d, id, memberId);
    });

    expect(doc.nodes[0]!.geometry!.width).toBe(ancho);
  });

  it("se edita un miembro por su id, este donde este", () => {
    const { doc: base, id } = conClase();

    const doc = run(base, (d) => {
      const operacion = boxOf(d, id).operations[0]!.id;
      updateClassMember(d, id, operacion, {
        name: "guardar",
        parameters: "p: Pedido",
        visibility: "protected",
      });
    });

    expect(boxOf(doc, id).operations[0]).toMatchObject({
      name: "guardar",
      parameters: "p: Pedido",
      visibility: "protected",
    });
  });

  it("borrar un miembro lo quita del compartimento que sea", () => {
    const { doc: base, id } = conClase();

    const doc = run(base, (d) => {
      removeClassMember(d, id, boxOf(d, id).operations[0]!.id);
    });

    expect(boxOf(doc, id).operations).toHaveLength(0);
    expect(boxOf(doc, id).attributes).toHaveLength(1);
  });

  it("se reordena dentro de su compartimento", () => {
    const { doc: base, id } = conClase();

    const conDos = run(base, (d) => {
      const nuevo = addClassMember(d, id, "attributes")!;
      updateClassMember(d, id, nuevo, { name: "segundo" });
    });

    const doc = run(conDos, (d) => {
      moveClassMember(d, id, boxOf(d, id).attributes[1]!.id, -1);
    });

    expect(boxOf(doc, id).attributes.map((m) => m.name)).toEqual([
      "segundo",
      "atributo",
    ]);
  });

  it("no se sale de los extremos al reordenar", () => {
    const { doc: base, id } = conClase();

    const doc = run(base, (d) => {
      moveClassMember(d, id, boxOf(d, id).attributes[0]!.id, -1);
    });

    expect(boxOf(doc, id).attributes).toHaveLength(1);
  });
});

describe("estereotipo", () => {
  it("ponerlo hace sitio encima del nombre", () => {
    const { doc: base, id } = conClase();
    const antes = base.nodes[0]!.geometry!.height;

    const doc = run(base, (d) => setClassStereotype(d, id, "interface"));

    expect(boxOf(doc, id).stereotype).toBe("interface");
    expect(doc.nodes[0]!.geometry!.height).toBeGreaterThan(antes);
  });

  it("uno en blanco es como no tener ninguno", () => {
    const { doc: base, id } = conClase();

    const doc = run(base, (d) => setClassStereotype(d, id, "   "));

    expect(boxOf(doc, id).stereotype).toBeUndefined();
  });
});

describe("nodos que no son clases", () => {
  it("las operaciones de clase no los tocan", () => {
    const sinClase = emptyClassDiagram();

    const doc = run(sinClase, (d) => {
      addClassMember(d, "fantasma", "attributes");
      removeClassMember(d, "fantasma", "x");
      setClassStereotype(d, "fantasma", "interface");
    });

    expect(doc).toEqual(sinClase);
  });
});
