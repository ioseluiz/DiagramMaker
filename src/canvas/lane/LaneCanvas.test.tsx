import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import { useDocumentStore } from "@/store/documentStore";
import { LaneCanvas } from "./LaneCanvas";
import { laneLabels, laneMeasures } from "./measure";

function doc(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "sequence",
    meta: { title: "Compra", created: 0, modified: 0 },
    nodes: [
      {
        id: "cli",
        label: "Cliente",
        shape: "rectangle",
        pinned: false,
        style: {},
        data: { type: "plain" },
      },
      {
        id: "api",
        label: "API",
        shape: "rectangle",
        pinned: false,
        style: {},
        data: { type: "plain" },
      },
    ],
    edges: [],
    layout: "auto",
    direction: "right",
    script: {
      steps: [
        {
          type: "message",
          id: "m1",
          from: "cli",
          to: "api",
          kind: "sync",
          text: "comprar",
          activate: true,
          deactivate: false,
        },
        {
          type: "block",
          id: "b1",
          kind: "alt",
          branches: [
            {
              id: "r1",
              label: "hay stock",
              steps: [
                {
                  type: "message",
                  id: "m2",
                  from: "api",
                  to: "cli",
                  kind: "reply",
                  text: "ok",
                  activate: false,
                  deactivate: true,
                },
              ],
            },
            { id: "r2", label: "no hay", steps: [] },
          ],
        },
        {
          type: "note",
          id: "n1",
          placement: "rightOf",
          participants: ["api"],
          text: "ojo",
        },
      ],
    },
  };
}

beforeEach(() => {
  useDocumentStore.setState({ selection: [] });
});

afterEach(cleanup);

describe("LaneCanvas", () => {
  it("dibuja una caja y una linea de vida por participante", () => {
    const { container } = render(<LaneCanvas document={doc()} />);

    expect(container.querySelectorAll(".lane-header")).toHaveLength(2);
    expect(container.querySelectorAll(".lane-lifeline")).toHaveLength(2);
    expect(screen.getByText("Cliente")).toBeInTheDocument();
  });

  it("dibuja los mensajes, el marco y la nota", () => {
    const { container } = render(<LaneCanvas document={doc()} />);

    expect(container.querySelectorAll(".lane-arrow")).toHaveLength(2);
    expect(container.querySelectorAll(".lane-frame")).toHaveLength(1);
    expect(container.querySelectorAll(".lane-note")).toHaveLength(1);
    expect(screen.getByText("comprar")).toBeInTheDocument();
  });

  it("el marco lleva su rotulo y el separador de la segunda rama", () => {
    const { container } = render(<LaneCanvas document={doc()} />);

    expect(screen.getByText("alt [hay stock]")).toBeInTheDocument();
    expect(screen.getByText("[no hay]")).toBeInTheDocument();
    expect(container.querySelectorAll(".lane-frame__divider")).toHaveLength(1);
  });

  it("un mensaje que activa dibuja su barra de ejecucion", () => {
    const { container } = render(<LaneCanvas document={doc()} />);

    expect(container.querySelectorAll(".lane-bar")).toHaveLength(1);
  });

  it("pinchar un mensaje lo selecciona", async () => {
    const { container } = render(<LaneCanvas document={doc()} />);

    const arrow = container.querySelector(".lane-arrow")!;
    await userEvent.click(arrow);

    expect(useDocumentStore.getState().selection).toEqual(["m1"]);
  });

  it("pinchar un participante lo selecciona", async () => {
    const { container } = render(<LaneCanvas document={doc()} />);

    await userEvent.click(container.querySelector(".lane-header")!);

    expect(useDocumentStore.getState().selection).toEqual(["cli"]);
  });

  it("pinchar el fondo deselecciona", async () => {
    useDocumentStore.setState({ selection: ["m1"] });
    const { container } = render(<LaneCanvas document={doc()} />);

    await userEvent.click(container.querySelector(".lane-canvas__surface")!);

    expect(useDocumentStore.getState().selection).toEqual([]);
  });

  it("lo seleccionado se marca en el dibujo", () => {
    useDocumentStore.setState({ selection: ["m1"] });
    const { container } = render(<LaneCanvas document={doc()} />);

    expect(container.querySelector(".lane-arrow[data-chosen]")).not.toBeNull();
  });

  it("sin participantes lo dice en vez de quedarse en blanco", () => {
    const vacio: Document = { ...doc(), nodes: [], script: { steps: [] } };
    render(<LaneCanvas document={vacio} />);

    expect(screen.getByText(/empieza por sus participantes/i)).toBeInTheDocument();
  });
});

describe("medidas del lienzo", () => {
  it("mide la cabecera de cada participante y el texto de cada paso", () => {
    const measures = laneMeasures(doc());

    expect(Object.keys(measures.headers).sort()).toEqual(["api", "cli"]);
    // El marco y sus ramas no empujan la geometria: solo mensajes y notas.
    expect(Object.keys(measures.texts).sort()).toEqual(["m1", "m2", "n1"]);
  });

  it("un mensaje sin texto no ocupa ancho", () => {
    const sinTexto = doc();
    const primero = sinTexto.script!.steps[0]!;
    if (primero.type === "message") primero.text = "";

    expect(laneMeasures(sinTexto).texts.m1).toBeUndefined();
  });

  it("las etiquetas incluyen marcos y separadores, que el reparto no mide", () => {
    const labels = laneLabels(doc());

    expect(Object.keys(labels).sort()).toEqual([
      "api",
      "b1",
      "cli",
      "m1",
      "m2",
      "n1",
      "r2",
    ]);
  });

  it("la primera rama se rotula en el marco, no en un separador", () => {
    // `r1` no aparece: su condicion va dentro del rotulo de `b1`.
    expect(laneLabels(doc()).r1).toBeUndefined();
    expect(laneLabels(doc()).b1!.lines[0]!.text).toBe("alt [hay stock]");
  });
});
