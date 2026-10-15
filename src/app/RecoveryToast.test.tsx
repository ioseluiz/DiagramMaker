import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { Document } from "@/model/generated/Document";
import { useDocumentStore } from "@/store/documentStore";
import { RecoveryToast } from "./RecoveryToast";

vi.mock("@/model/api", async () => {
  const actual = await vi.importActual<typeof import("@/model/api")>("@/model/api");
  return { ...actual, autosaveClear: vi.fn().mockResolvedValue(undefined) };
});

const api = await import("@/model/api");

function emptyDoc(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "architecture",
    meta: { title: "Prueba", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "manual",
    direction: "right",
  };
}

function withRecovery(savedAt: number, path?: string) {
  useDocumentStore.setState({
    recovery: { document: emptyDoc(), savedAt, path },
  });
}

afterEach(() => {
  useDocumentStore.setState({ recovery: null });
  vi.clearAllMocks();
});

describe("RecoveryToast", () => {
  it("no aparece si no hay copia pendiente", () => {
    const { container } = render(<RecoveryToast />);

    expect(container).toBeEmptyDOMElement();
  });

  it("muestra el tiempo relativo, no una fecha completa", () => {
    withRecovery(Date.now() - 12 * 60_000, "C:\\proyectos\\esquema.dgm");

    render(<RecoveryToast />);
    const detalle = screen.getByText(/esquema\.dgm/).textContent ?? "";

    expect(detalle).toMatch(/12/);
    // Una fecha larga traeria la barra o los dos puntos de la hora.
    expect(detalle).not.toMatch(/\d{4}/);
    expect(detalle).not.toMatch(/\d{2}:\d{2}/);
  });

  it("el detalle es corto: el aviso tiene que caber en una tarjeta pequena", () => {
    withRecovery(Date.now() - 3 * 3_600_000, "C:\\proyectos\\esquema.dgm");

    render(<RecoveryToast />);

    expect((screen.getByText(/esquema\.dgm/).textContent ?? "").length).toBeLessThan(
      40,
    );
  });

  it("indica cuando la copia es de un diagrama nunca guardado", () => {
    withRecovery(Date.now() - 60_000);

    render(<RecoveryToast />);

    // El titulo tambien dice «sin guardar»; lo que se comprueba es el detalle,
    // que es el que lleva el separador.
    expect(screen.getByText(/·\s*sin guardar/)).toBeInTheDocument();
  });

  it("recuperar carga el documento de la copia y cierra el aviso", async () => {
    withRecovery(Date.now() - 60_000, "C:\\x.dgm");

    render(<RecoveryToast />);
    await userEvent.click(screen.getByRole("button", { name: "Recuperar" }));

    const state = useDocumentStore.getState();
    expect(state.recovery).toBeNull();
    expect(state.history?.present.id).toBe("d1");
    // Recuperado no es guardado: sigue habiendo trabajo sin escribir a disco.
    expect(state.dirty).toBe(true);
  });

  it("cerrar con la × esconde el aviso pero NO borra la copia", async () => {
    withRecovery(Date.now() - 60_000, "C:\\x.dgm");

    render(<RecoveryToast />);
    await userEvent.click(screen.getByRole("button", { name: "Cerrar aviso" }));

    expect(useDocumentStore.getState().recovery).toBeNull();
    expect(api.autosaveClear).not.toHaveBeenCalled();
  });

  it("descartar si borra la copia del disco", async () => {
    withRecovery(Date.now() - 60_000, "C:\\x.dgm");

    render(<RecoveryToast />);
    await userEvent.click(screen.getByRole("button", { name: "Descartar" }));

    expect(useDocumentStore.getState().recovery).toBeNull();
    expect(api.autosaveClear).toHaveBeenCalledOnce();
  });
});
