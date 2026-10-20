import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import { initHistory } from "@/store/history";
import { useDocumentStore } from "@/store/documentStore";
import { CommandPalette } from "./CommandPalette";
import { useCanvasTool } from "./useCanvasTool";
import { useCodePanel } from "./useCodePanel";
import { useCommandPalette } from "./useCommandPalette";
import { useDialogs } from "./useDialogs";

function doc(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "architecture",
    meta: { title: "prueba", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "manual",
    direction: "right",
  };
}

/** Monta la paleta y la abre, que es como se usa siempre. */
async function open(user: ReturnType<typeof userEvent.setup>) {
  render(<CommandPalette />);
  await user.keyboard("{Control>}k{/Control}");
}

function rows(): string[] {
  return screen.queryAllByRole("option").map((el) => el.textContent ?? "");
}

function selected(): string {
  return screen.getByRole("option", { selected: true }).textContent ?? "";
}

beforeEach(() => {
  useCanvasTool.setState({ tool: "select" });
  useCommandPalette.setState({ open: false });
  useCodePanel.setState({ open: false });
  useDialogs.setState({ dialog: null });
  useDocumentStore.setState({ history: initHistory(doc()), path: null, dirty: false });
});

afterEach(cleanup);

describe("CommandPalette", () => {
  it("no se ve hasta que se pide", () => {
    render(<CommandPalette />);

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("Ctrl+K la abre y la vuelve a cerrar", async () => {
    const user = userEvent.setup();
    await open(user);
    expect(screen.getByRole("dialog")).toBeInTheDocument();

    await user.keyboard("{Control>}k{/Control}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("Ctrl+Shift+P tambien la abre", async () => {
    const user = userEvent.setup();
    render(<CommandPalette />);

    await user.keyboard("{Control>}{Shift>}P{/Shift}{/Control}");

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("empieza con todo y el foco en la busqueda", async () => {
    const user = userEvent.setup();
    await open(user);

    expect(screen.getByLabelText("Buscar un comando")).toHaveFocus();
    expect(rows().length).toBeGreaterThan(5);
  });

  it("filtra segun se escribe", async () => {
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("exportar");

    expect(rows()).toHaveLength(1);
    expect(rows()[0]).toContain("Exportar");
  });

  it("encuentra sin tildes", async () => {
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("codigo");

    expect(rows().some((r) => r.includes("código"))).toBe(true);
  });

  it("lo dice cuando no encaja nada", async () => {
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("zzzz");

    expect(rows()).toHaveLength(0);
    expect(screen.getByText(/no hay ningún comando/i)).toBeInTheDocument();
  });

  it("las flechas mueven la fila elegida", async () => {
    const user = userEvent.setup();
    await open(user);
    const primera = selected();

    await user.keyboard("{ArrowDown}");
    const segunda = selected();

    expect(segunda).not.toBe(primera);

    await user.keyboard("{ArrowUp}");
    expect(selected()).toBe(primera);
  });

  it("Intro ejecuta el comando elegido y cierra", async () => {
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("mano");
    expect(selected()).toContain("mano");
    await user.keyboard("{Enter}");

    expect(useCanvasTool.getState().tool).toBe("hand");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
  });

  it("un clic tambien lo ejecuta", async () => {
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("exportar");
    await user.click(screen.getByRole("option", { name: /exportar/i }));

    expect(useDialogs.getState().dialog).toBe("export");
  });

  it("Escape cierra sin ejecutar nada", async () => {
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("mano");
    await user.keyboard("{Escape}");

    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("un comando apagado se ve pero no se ejecuta", async () => {
    // Sin nada que deshacer, "Deshacer" sale en la lista —para que se vea que
    // existe y con que atajo— pero no hace nada.
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("deshacer");
    const fila = screen.getByRole("option", { name: /deshacer/i });
    expect(fila).toHaveAttribute("aria-disabled", "true");

    await user.click(fila);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
  });

  it("muestra el atajo de los comandos que lo tienen", async () => {
    const user = userEvent.setup();
    await open(user);

    await user.keyboard("guardar como");

    expect(rows()[0]).toContain("Ctrl+Shift+S");
  });

  it("empieza limpia cada vez que se abre", async () => {
    const user = userEvent.setup();
    await open(user);
    await user.keyboard("exportar");
    await user.keyboard("{Escape}");

    await user.keyboard("{Control>}k{/Control}");

    expect(screen.getByLabelText("Buscar un comando")).toHaveValue("");
    expect(rows().length).toBeGreaterThan(5);
  });
});
