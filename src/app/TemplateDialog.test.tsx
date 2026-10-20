import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { Template } from "@/model/generated/Template";
import { useDocumentStore } from "@/store/documentStore";
import { TemplateDialog } from "./TemplateDialog";

const templateList = vi.fn<() => Promise<Template[]>>();
const importCode = vi.fn<(...args: unknown[]) => Promise<unknown>>();

vi.mock("@/model/api", async () => {
  const actual = await vi.importActual<typeof import("@/model/api")>("@/model/api");
  return {
    ...actual,
    templateList: () => templateList(),
    importCode: (...args: unknown[]) => importCode(...args),
  };
});

// ELK corre en un worker que jsdom no tiene; aqui solo estorba.
vi.mock("@/layout/autoLayout", () => ({
  applyAutoLayout: vi.fn().mockResolvedValue(undefined),
}));

function doc(): Document {
  return {
    schemaVersion: 1,
    id: "importado",
    kind: "state",
    meta: { title: "Ciclo de vida de un pedido", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "auto",
    direction: "right",
  };
}

const PLANTILLAS: Template[] = [
  {
    id: "er-tienda",
    name: "Tienda en linea",
    description: "Clientes, pedidos y lineas.",
    kind: "er",
    language: "dbml",
    source: "Table clientes {\n  id integer [pk]\n}",
  },
  {
    id: "estado-pedido",
    name: "Ciclo de vida de un pedido",
    description: "Estados y transiciones.",
    kind: "state",
    language: "plantUml",
    source: "@startuml\n[*] --> Nuevo\n@enduml",
  },
];

const onClose = vi.fn();

beforeEach(() => {
  templateList.mockReset().mockResolvedValue(PLANTILLAS);
  importCode.mockReset().mockResolvedValue({ document: doc(), diagnostics: [] });
  onClose.mockReset();
  useDocumentStore.setState({ history: null, imported: [] });
});

afterEach(cleanup);

/** Monta el dialogo y espera a que lleguen las plantillas. */
async function abrir() {
  render(<TemplateDialog onClose={onClose} />);
  await screen.findByRole("button", { name: /tienda en linea/i });
}

describe("TemplateDialog", () => {
  it("lista lo que ofrece el backend, con su tipo y su explicacion", async () => {
    await abrir();

    const fila = screen.getByRole("button", { name: /ciclo de vida/i });
    expect(fila).toHaveTextContent("Estado");
    expect(fila).toHaveTextContent("Estados y transiciones.");
  });

  it("abrir una plantilla la importa y sustituye el documento", async () => {
    const user = userEvent.setup();
    await abrir();

    await user.click(screen.getByRole("button", { name: /ciclo de vida/i }));

    await waitFor(() => {
      expect(useDocumentStore.getState().history?.present.id).toBe("importado");
    });
    expect(importCode).toHaveBeenCalledWith(
      "plantUml",
      PLANTILLAS[1]!.source,
      expect.any(Number),
    );
  });

  it("se cierra al abrir una", async () => {
    const user = userEvent.setup();
    await abrir();

    await user.click(screen.getByRole("button", { name: /tienda en linea/i }));

    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });

  it("el documento que llega es el importado, no uno en blanco", async () => {
    const user = userEvent.setup();
    await abrir();

    await user.click(screen.getByRole("button", { name: /ciclo de vida/i }));

    await waitFor(() => {
      expect(useDocumentStore.getState().history?.present.meta.title).toBe(
        "Ciclo de vida de un pedido",
      );
    });
  });

  it("un fallo del backend se dice y no cierra el dialogo", async () => {
    const user = userEvent.setup();
    importCode.mockRejectedValue(new Error("no se pudo"));
    await abrir();

    await user.click(screen.getByRole("button", { name: /tienda en linea/i }));

    expect(await screen.findByText(/no se pudo/)).toBeInTheDocument();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("si no se pueden pedir las plantillas lo dice", async () => {
    templateList.mockRejectedValue(new Error("sin lista"));
    render(<TemplateDialog onClose={onClose} />);

    expect(await screen.findByText(/sin lista/)).toBeInTheDocument();
  });

  it("Escape cierra", async () => {
    const user = userEvent.setup();
    await abrir();

    await user.keyboard("{Escape}");

    expect(onClose).toHaveBeenCalled();
  });

  it("Cancelar cierra sin abrir nada", async () => {
    const user = userEvent.setup();
    await abrir();

    await user.click(screen.getByRole("button", { name: "Cancelar" }));

    expect(onClose).toHaveBeenCalled();
    expect(importCode).not.toHaveBeenCalled();
  });
});
