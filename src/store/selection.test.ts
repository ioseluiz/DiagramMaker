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
