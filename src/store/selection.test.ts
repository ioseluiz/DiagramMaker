import { describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import { initHistory } from "./history";
import { useDocumentStore } from "./documentStore";

function emptyDoc(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "architecture",
    meta: { title: "t", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "manual",
    direction: "right",
  };
}

describe("select", () => {
  it("no cambia la identidad de la seleccion si los ids son los mismos", () => {
    // Regresion: React Flow emite su evento de seleccion en cada repintado.
    // Escribir un array nuevo cada vez recalculaba todos los nodos y aristas
    // del lienzo y disparaba otro repintado — latencia visible al pulsar.
    useDocumentStore.setState({ history: initHistory(emptyDoc()), selection: [] });

    useDocumentStore.getState().select(["a", "b"]);
    const first = useDocumentStore.getState().selection;

    useDocumentStore.getState().select(["a", "b"]);

    expect(useDocumentStore.getState().selection).toBe(first);
  });

  it("si cambia algun id, si actualiza", () => {
    useDocumentStore.setState({ selection: [] });

    useDocumentStore.getState().select(["a"]);
    const first = useDocumentStore.getState().selection;
    useDocumentStore.getState().select(["b"]);

    expect(useDocumentStore.getState().selection).not.toBe(first);
    expect(useDocumentStore.getState().selection).toEqual(["b"]);
  });

  it("distingue un orden distinto y un tamano distinto", () => {
    useDocumentStore.setState({ selection: ["a", "b"] });

    useDocumentStore.getState().select(["b", "a"]);
    expect(useDocumentStore.getState().selection).toEqual(["b", "a"]);

    useDocumentStore.getState().select(["b"]);
    expect(useDocumentStore.getState().selection).toEqual(["b"]);
  });

  it("vaciar la seleccion dos veces tampoco escribe", () => {
    useDocumentStore.setState({ selection: [] });
    const empty = useDocumentStore.getState().selection;

    useDocumentStore.getState().select([]);

    expect(useDocumentStore.getState().selection).toBe(empty);
  });
});

describe("applySelection", () => {
  /** Deja el store listo y sin nada seleccionado. */
  function limpio() {
    useDocumentStore.setState({ history: initHistory(emptyDoc()), selection: [] });
    return useDocumentStore.getState();
  }

  it("selecciona lo que el lienzo dice que se ha seleccionado", () => {
    // El lienzo esta en modo controlado: React Flow no se guarda la
    // seleccion, la manda como cambios y espera recibirla de vuelta.
    // Ignorarlos dejaba el clic sin efecto, ni en nodos ni en relaciones.
    limpio().applySelection([{ id: "e1", selected: true }]);

    expect(useDocumentStore.getState().selection).toEqual(["e1"]);
  });

  it("y suelta lo que dice que se ha soltado", () => {
    limpio().applySelection([{ id: "a", selected: true }]);
    useDocumentStore.getState().applySelection([{ id: "a", selected: false }]);

    expect(useDocumentStore.getState().selection).toEqual([]);
  });

  it("junta las dos llamadas de un mismo gesto", () => {
    // Al pulsar una relacion, React Flow avisa en dos tandas: una dice que
    // se seleccione la arista y otra que se suelten los nodos. Si la segunda
    // partiera de la seleccion que habia al renderizar en vez de la de ese
    // momento, pisaria a la primera y el clic acabaria sin seleccionar nada.
    const store = limpio();
    store.applySelection([{ id: "n1", selected: true }]);

    useDocumentStore.getState().applySelection([{ id: "e1", selected: true }]);
    useDocumentStore.getState().applySelection([{ id: "n1", selected: false }]);

    expect(useDocumentStore.getState().selection).toEqual(["e1"]);
  });

  it("sin cambios no toca nada", () => {
    // Llega una tanda vacia en cada repintado; escribir siempre haria
    // recalcular el lienzo entero sin motivo.
    limpio().applySelection([{ id: "a", selected: true }]);
    const antes = useDocumentStore.getState().selection;

    useDocumentStore.getState().applySelection([]);

    expect(useDocumentStore.getState().selection).toBe(antes);
  });

  it("repetir la misma seleccion conserva la identidad del array", () => {
    limpio().applySelection([{ id: "a", selected: true }]);
    const antes = useDocumentStore.getState().selection;

    useDocumentStore.getState().applySelection([{ id: "a", selected: true }]);

    expect(useDocumentStore.getState().selection).toBe(antes);
  });

  it("admite varios a la vez sin repetirlos", () => {
    limpio().applySelection([
      { id: "a", selected: true },
      { id: "b", selected: true },
      { id: "a", selected: true },
    ]);

    expect(useDocumentStore.getState().selection).toEqual(["a", "b"]);
  });
});
