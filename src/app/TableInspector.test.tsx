import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { ErTable } from "@/model/generated/ErTable";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { addColumn, addTable } from "@/store/operations";
import { initHistory } from "@/store/history";
import { TableInspector } from "./TableInspector";

function erDocument(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "er",
    meta: { title: "Prueba", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "manual",
    direction: "right",
  };
}

/** Monta el inspector sobre una tabla recien creada en el store. */
function mount(extraColumns = 0) {
  let id = "";
  useDocumentStore.setState({ history: initHistory(erDocument()) });

  useDocumentStore.getState().edit("preparar", (d) => {
    id = addTable(d, { x: 0, y: 0, label: "users" });
    for (let i = 0; i < extraColumns; i++) addColumn(d, id);
  });

  function Harness() {
    const doc = useDocumentStore(selectDocument)!;
    const node = doc.nodes.find((n) => n.id === id)!;
    return <TableInspector node={node} table={node.data as ErTable} />;
  }

  render(<Harness />);
  return { id, table: () => currentTable(id) };
}

function currentTable(id: string): ErTable {
  const doc = selectDocument(useDocumentStore.getState())!;
  const node = doc.nodes.find((n) => n.id === id)!;
  if (node.data.type !== "er") throw new Error("no es una tabla");
  return node.data;
}

beforeEach(() => {
  useDocumentStore.setState({ history: null, selection: [] });
});

afterEach(() => {
  // Desmontar antes de vaciar el store: si no, el componente se repinta
  // contra un documento que ya no existe.
  cleanup();
  useDocumentStore.setState({ history: null, selection: [] });
});

describe("TableInspector", () => {
  it("muestra el nombre de la tabla y su cuenta de columnas", () => {
    mount();

    expect(screen.getByDisplayValue("users")).toBeInTheDocument();
    expect(screen.getByText("Columnas (1)")).toBeInTheDocument();
  });

  it("anade una columna al pulsar el boton", async () => {
    const { table } = mount();

    await userEvent.click(screen.getByRole("button", { name: /Añadir/ }));

    expect(table().columns).toHaveLength(2);
  });

  it("renombra una columna al escribir", async () => {
    const { table } = mount();

    const input = screen.getByLabelText("Nombre de la columna");
    await userEvent.clear(input);
    await userEvent.type(input, "clave");

    expect(table().columns[0]!.name).toBe("clave");
  });

  it("cambia el tipo de una columna", async () => {
    const { table } = mount();

    const input = screen.getByLabelText("Tipo de la columna");
    await userEvent.clear(input);
    await userEvent.type(input, "bigint");

    expect(table().columns[0]!.dataType).toBe("bigint");
  });

  it("los interruptores reflejan el estado de la columna", () => {
    mount();

    // La columna `id` nace como clave primaria.
    expect(screen.getByRole("button", { name: "PK" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "U" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("NN esta deshabilitado mientras la columna sea clave primaria", async () => {
    const { table } = mount();

    // Una PK ya es no nula; marcarlo aparte no significa nada.
    expect(screen.getByRole("button", { name: "NN" })).toBeDisabled();

    await userEvent.click(screen.getByRole("button", { name: "PK" }));

    expect(table().columns[0]!.primaryKey).toBe(false);
    expect(screen.getByRole("button", { name: "NN" })).toBeEnabled();
  });

  it("los interruptores van en los dos sentidos", async () => {
    const { table } = mount();

    // `unique` empieza apagado.
    await userEvent.click(screen.getByRole("button", { name: "U" }));
    expect(table().columns[0]!.unique).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "U" }));
    expect(table().columns[0]!.unique).toBe(false);

    // `increment` empieza encendido: la columna `id` nace autoincremental.
    expect(table().columns[0]!.increment).toBe(true);
    await userEvent.click(screen.getByRole("button", { name: "AI" }));
    expect(table().columns[0]!.increment).toBe(false);
  });

  it("borra una columna", async () => {
    const { table } = mount(1);
    const [primera] = table().columns;

    await userEvent.click(
      screen.getByRole("button", { name: `Borrar ${primera!.name}` }),
    );

    expect(table().columns).toHaveLength(1);
    expect(table().columns[0]!.name).not.toBe(primera!.name);
  });

  it("reordena columnas con las flechas", async () => {
    const { table } = mount(1);
    const nombres = table().columns.map((c) => c.name);

    await userEvent.click(screen.getByRole("button", { name: `Bajar ${nombres[0]}` }));

    expect(table().columns.map((c) => c.name)).toEqual([nombres[1], nombres[0]]);
  });

  it("la primera columna no se puede subir y la ultima no se puede bajar", () => {
    const { table } = mount(1);
    const [primera, ultima] = table().columns;

    expect(
      screen.getByRole("button", { name: `Subir ${primera!.name}` }),
    ).toBeDisabled();
    expect(
      screen.getByRole("button", { name: `Bajar ${ultima!.name}` }),
    ).toBeDisabled();
  });

  it("guarda alias y nota", async () => {
    const { id } = mount();

    await userEvent.type(screen.getByLabelText("Alias"), "U");
    expect(currentTable(id).alias).toBe("U");

    await userEvent.type(screen.getByLabelText("Nota"), "Clientes");
    expect(currentTable(id).note).toBe("Clientes");
  });

  it("una tabla sin columnas lo explica en vez de dejar un hueco", async () => {
    const { table } = mount();

    await userEvent.click(screen.getByRole("button", { name: "Borrar id" }));

    expect(table().columns).toHaveLength(0);
    expect(screen.getByText(/caja con su nombre/)).toBeInTheDocument();
  });

  it("cada fila trae sus propios controles", () => {
    mount(2);

    const filas = screen.getAllByRole("listitem");
    expect(filas).toHaveLength(3);

    for (const fila of filas) {
      expect(within(fila).getByLabelText("Nombre de la columna")).toBeInTheDocument();
      expect(within(fila).getByRole("button", { name: "PK" })).toBeInTheDocument();
    }
  });
});
