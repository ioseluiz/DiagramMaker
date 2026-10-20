import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { Document } from "@/model/generated/Document";
import { initHistory } from "@/store/history";
import { useDocumentStore } from "@/store/documentStore";
import { CodePanel } from "./CodePanel";
import { useCodePanel } from "./useCodePanel";

const emitCode = vi.fn<(...args: unknown[]) => Promise<string>>();

vi.mock("@/model/api", async () => {
  const actual = await vi.importActual<typeof import("@/model/api")>("@/model/api");
  return {
    ...actual,
    emitCode: (...args: unknown[]) => emitCode(...args),
    emitCodeToFile: vi.fn().mockResolvedValue(undefined),
  };
});

function doc(kind: DiagramKind): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind,
    meta: { title: "tienda", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "manual",
    direction: "right",
  };
}

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
  emitCode.mockReset();
  emitCode.mockResolvedValue("Table users {\n  id integer [pk]\n}\n");
  useCodePanel.setState({ open: true });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  useDocumentStore.setState({ history: null });
});

/**
 * Adelanta el reloj dejando que React aplique lo que llegue.
 *
 * El codigo se pide con una promesa y se guarda en estado al resolverse; sin
 * `act` esa actualizacion no se pinta y las consultas no encuentran nada.
 */
async function tick(ms: number) {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

/** Monta el panel y deja pasar el tiempo de espera de la regeneracion. */
async function mount(kind: DiagramKind = "er") {
  useDocumentStore.setState({ history: initHistory(doc(kind)) });
  render(<CodePanel />);
  await tick(300);
}

describe("CodePanel", () => {
  it("muestra el codigo generado del documento", async () => {
    await mount("er");

    expect(screen.getByText(/Table users/)).toBeInTheDocument();
    expect(emitCode).toHaveBeenCalledWith(
      "dbml",
      expect.objectContaining({ kind: "er" }),
    );
  });

  it("espera antes de regenerar: no llama por cada cambio", async () => {
    useDocumentStore.setState({ history: initHistory(doc("er")) });
    render(<CodePanel />);

    // Antes de cumplirse la espera no se ha pedido nada.
    await tick(100);
    expect(emitCode).not.toHaveBeenCalled();

    await tick(200);
    expect(emitCode).toHaveBeenCalledTimes(1);
  });

  it("regenera cuando cambia el documento", async () => {
    await mount("er");
    expect(emitCode).toHaveBeenCalledTimes(1);

    act(() => {
      useDocumentStore.getState().edit("titulo", (d) => {
        d.meta.title = "otro";
      });
    });
    await tick(300);

    expect(emitCode).toHaveBeenCalledTimes(2);
  });

  it("un diagrama con un solo generador no ofrece donde elegir", async () => {
    await mount("er");

    expect(screen.queryByLabelText("Lenguaje del código")).not.toBeInTheDocument();
  });

  it("arquitectura deja elegir entre Mermaid y PlantUML", async () => {
    await mount("architecture");
    expect(emitCode).toHaveBeenCalledWith("mermaid", expect.anything());

    await userEvent.selectOptions(
      screen.getByLabelText("Lenguaje del código"),
      "plantUml",
    );
    await tick(300);

    expect(emitCode).toHaveBeenLastCalledWith("plantUml", expect.anything());
  });

  it("un tipo de diagrama sin generador lo dice en vez de quedarse vacio", async () => {
    // El caso va cambiando segun llegan los generadores. Cuando los tengan
    // todos, esta prueba no se mueve otra vez: se borra, porque el aviso
    // dejara de poder aparecer.
    await mount("activity");

    expect(screen.getByText(/no hay generador de código/i)).toBeInTheDocument();
    expect(emitCode).not.toHaveBeenCalled();
  });

  it("copia al portapapeles y avisa", async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    Object.assign(navigator, { clipboard: { writeText } });

    await mount("er");
    await userEvent.click(screen.getByRole("button", { name: "Copiar" }));

    expect(writeText).toHaveBeenCalledWith("Table users {\n  id integer [pk]\n}\n");
    expect(screen.getByRole("button", { name: "Copiado" })).toBeInTheDocument();
  });

  it("el aviso de copiado se retira solo", async () => {
    Object.assign(navigator, {
      clipboard: { writeText: vi.fn().mockResolvedValue(undefined) },
    });

    await mount("er");
    await userEvent.click(screen.getByRole("button", { name: "Copiar" }));
    await tick(2_000);

    expect(screen.getByRole("button", { name: "Copiar" })).toBeInTheDocument();
  });

  it("un fallo al generar se muestra sin tumbar el panel", async () => {
    emitCode.mockRejectedValue({ kind: "unsupported", message: "no se pudo" });

    await mount("er");

    expect(screen.getByText("no se pudo")).toBeInTheDocument();
  });

  it("cerrar oculta el panel", async () => {
    await mount("er");

    await userEvent.click(screen.getByLabelText("Cerrar el panel de código"));

    expect(useCodePanel.getState().open).toBe(false);
  });
});
