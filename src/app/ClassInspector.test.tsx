import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";

import type { ClassBox } from "@/model/generated/ClassBox";
import type { Document } from "@/model/generated/Document";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { addClass } from "@/store/operations";
import { initHistory } from "@/store/history";
import { ClassInspector } from "./ClassInspector";

function classDocument(): Document {
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

function currentBox(id: string): ClassBox {
  const doc = useDocumentStore.getState().history!.present;
  const node = doc.nodes.find((n) => n.id === id)!;
  if (node.data.type !== "class") throw new Error("no es una clase");
  return node.data;
}

/** Monta el inspector sobre una clase recien creada en el store. */
function mount() {
  let id = "";
  useDocumentStore.setState({ history: initHistory(classDocument()) });

  useDocumentStore.getState().edit("preparar", (d) => {
    id = addClass(d, { x: 0, y: 0, label: "Pedido" });
  });

  function Harness() {
    const doc = useDocumentStore(selectDocument)!;
    const node = doc.nodes.find((n) => n.id === id)!;
    return <ClassInspector node={node} box={node.data as ClassBox} />;
  }

  render(<Harness />);
  return { id, box: () => currentBox(id) };
}

/** La lista de un compartimento, por su encabezado. */
function compartment(title: string): HTMLElement {
  const head = screen.getByText(new RegExp(`^${title} \\(`));
  return head.closest(".members__head")!.nextElementSibling as HTMLElement;
}

afterEach(() => {
  cleanup();
  useDocumentStore.setState({ history: null });
});

describe("ClassInspector", () => {
  it("muestra los dos compartimentos con su cuenta", () => {
    mount();

    expect(screen.getByText("Atributos (1)")).toBeInTheDocument();
    expect(screen.getByText("Operaciones (1)")).toBeInTheDocument();
  });

  it("renombrar la clase llega al documento", async () => {
    const { id } = mount();
    const campo = screen.getByDisplayValue("Pedido");

    await userEvent.clear(campo);
    await userEvent.type(campo, "Factura");

    const doc = useDocumentStore.getState().history!.present;
    expect(doc.nodes.find((n) => n.id === id)!.label).toBe("Factura");
  });

  it("anade un atributo al compartimento que toca", async () => {
    const { box } = mount();

    const head = screen.getByText("Atributos (1)").closest(".members__head")!;
    await userEvent.click(within(head as HTMLElement).getByText("+ Añadir"));

    expect(box().attributes).toHaveLength(2);
    expect(box().operations).toHaveLength(1);
  });

  it("anade una operacion al suyo", async () => {
    const { box } = mount();

    const head = screen.getByText("Operaciones (1)").closest(".members__head")!;
    await userEvent.click(within(head as HTMLElement).getByText("+ Añadir"));

    expect(box().operations).toHaveLength(2);
    expect(box().attributes).toHaveLength(1);
  });

  it("cambia la visibilidad de un miembro", async () => {
    const { box } = mount();

    await userEvent.selectOptions(
      screen.getByLabelText("Visibilidad de atributo"),
      "protected",
    );

    expect(box().attributes[0]!.visibility).toBe("protected");
  });

  it("solo las operaciones piden parametros", () => {
    // Un atributo no los tiene, y ofrecer el campo invitaria a escribir algo
    // que no se dibujaria en ninguna parte.
    mount();

    const campos = screen.getAllByPlaceholderText(/parametros/i);
    expect(campos).toHaveLength(1);
  });

  it("marcar estatico y abstracto llega al documento", async () => {
    const { box } = mount();

    const fila = compartment("Atributos").querySelector(".members__row")!;
    await userEvent.click(within(fila as HTMLElement).getByTitle(/Estatico/));
    await userEvent.click(within(fila as HTMLElement).getByTitle(/Abstracto/));

    expect(box().attributes[0]).toMatchObject({ isStatic: true, isAbstract: true });
  });

  it("el estereotipo llega al documento y se limpia si queda en blanco", async () => {
    const { box } = mount();
    const campo = screen.getByPlaceholderText("opcional");

    await userEvent.type(campo, "interface");
    expect(box().stereotype).toBe("interface");

    await userEvent.clear(campo);
    expect(box().stereotype).toBeUndefined();
  });

  it("borrar un miembro lo quita", async () => {
    const { box } = mount();

    const fila = compartment("Operaciones").querySelector(".members__row")!;
    await userEvent.click(within(fila as HTMLElement).getByTitle("Borrar miembro"));

    expect(box().operations).toHaveLength(0);
  });

  it("el primero no se puede subir y el ultimo no se puede bajar", async () => {
    mount();

    const fila = compartment("Atributos").querySelector(".members__row")!;
    expect(within(fila as HTMLElement).getByTitle("Subir")).toBeDisabled();
    expect(within(fila as HTMLElement).getByTitle("Bajar")).toBeDisabled();
  });

  it("con dos miembros se pueden reordenar", async () => {
    const { box } = mount();

    const head = screen.getByText("Atributos (1)").closest(".members__head")!;
    await userEvent.click(within(head as HTMLElement).getByText("+ Añadir"));

    // Los dos nacen llamandose igual, asi que lo que se sigue es el id.
    const antes = box().attributes.map((m) => m.id);

    const filas = compartment("Atributos").querySelectorAll(".members__row");
    await userEvent.click(within(filas[1] as HTMLElement).getByTitle("Subir"));

    expect(box().attributes.map((m) => m.id)).toEqual([antes[1], antes[0]]);
  });
});
