import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { Report } from "@/model/generated/Report";
import type { Template } from "@/model/generated/Template";
import { useDocumentStore } from "@/store/documentStore";
import { App } from "./App";
import { useCodePanel } from "./useCodePanel";
import { useCommandPalette } from "./useCommandPalette";
import { useDiagnostics } from "./useDiagnostics";
import { useDialogs } from "./useDialogs";

/**
 * El recorrido por la interfaz, con la aplicación entera montada.
 *
 * Las demás pruebas del frontend montan un componente cada una. Esta monta
 * `App` con su store de verdad y recorre lo que haría alguien: abrir un
 * ejemplo, ver el diagrama, mirar su código, abrir el panel de problemas y
 * exportar. Lo que busca es el cableado —barra → diálogo → store → lienzo y
 * paneles—, que es justo lo que no se ve probando piezas sueltas.
 *
 * Lo único simulado es el puente con Rust y el auto-layout; de esa mitad se
 * encarga `src-tauri/tests/recorrido.rs`, que hace el mismo viaje por el
 * backend.
 *
 * El ejemplo es una secuencia a propósito: su lienzo es SVG propio y se
 * monta en jsdom. React Flow necesitaría un navegador de verdad, y eso es lo
 * que cubriría `tauri-driver` (ver `docs/pruebas-e2e.md`).
 */

const api = {
  documentNew: vi.fn<(...a: unknown[]) => Promise<Document>>(),
  documentValidate: vi.fn<(...a: unknown[]) => Promise<Report>>(),
  templateList: vi.fn<() => Promise<Template[]>>(),
  importCode: vi.fn<(...a: unknown[]) => Promise<unknown>>(),
  emitCode: vi.fn<(...a: unknown[]) => Promise<string>>(),
  recentList: vi.fn<() => Promise<unknown[]>>(),
  autosaveRead: vi.fn<() => Promise<null>>(),
  autosaveWrite: vi.fn<() => Promise<void>>(),
  appInfo: vi.fn<() => Promise<unknown>>(),
};

vi.mock("@/model/api", async () => {
  const actual = await vi.importActual<typeof import("@/model/api")>("@/model/api");
  return {
    ...actual,
    documentNew: (...a: unknown[]) => api.documentNew(...a),
    documentValidate: (...a: unknown[]) => api.documentValidate(...a),
    templateList: () => api.templateList(),
    importCode: (...a: unknown[]) => api.importCode(...a),
    emitCode: (...a: unknown[]) => api.emitCode(...a),
    emitCodeToFile: vi.fn().mockResolvedValue(undefined),
    recentList: () => api.recentList(),
    recentRemember: vi.fn().mockResolvedValue([]),
    autosaveRead: () => api.autosaveRead(),
    autosaveWrite: () => api.autosaveWrite(),
    appInfo: () => api.appInfo(),
  };
});

vi.mock("@/layout/autoLayout", () => ({
  applyAutoLayout: vi.fn().mockResolvedValue(undefined),
}));

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ setTitle: vi.fn().mockResolvedValue(undefined) }),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn().mockResolvedValue(undefined),
}));

// React Flow mide y transforma con el navegador: su `ZoomPane` revienta en
// jsdom. Se sustituye por una marca, que es justo el limite de esta prueba.
// Lo de dentro del lienzo de grafo solo lo cubriria `tauri-driver`.
vi.mock("@/canvas/graph/GraphCanvas", () => ({
  GraphCanvas: () => <div data-testid="lienzo-de-grafo" />,
}));

/* --- Lo que devuelve el backend simulado ---------------------------------- */

function vacio(): Document {
  return {
    schemaVersion: 1,
    id: "en-blanco",
    kind: "architecture",
    meta: { title: "Arquitectura", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "auto",
    direction: "right",
  };
}

/** Una secuencia con dos participantes y un mensaje. */
function secuencia(): Document {
  return {
    schemaVersion: 1,
    id: "login",
    kind: "sequence",
    meta: { title: "Inicio de sesión", created: 0, modified: 0 },
    nodes: [
      {
        id: "nav",
        label: "Navegador",
        shape: "rectangle",
        pinned: false,
        style: {},
        data: { type: "plain" },
      },
      {
        id: "svc",
        label: "Servicio",
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
          from: "nav",
          to: "svc",
          kind: "sync",
          text: "credenciales",
          activate: false,
          deactivate: false,
        },
      ],
    },
  };
}

const PLANTILLA: Template = {
  id: "secuencia-login",
  name: "Inicio de sesión",
  description: "Mensajes y activaciones.",
  kind: "sequence",
  language: "plantUml",
  source: "@startuml\nnav -> svc : credenciales\n@enduml",
};

const LIMPIO: Report = { ok: true, diagnostics: [] };

beforeEach(() => {
  for (const fn of Object.values(api)) fn.mockReset();

  api.documentNew.mockResolvedValue(vacio());
  api.documentValidate.mockResolvedValue(LIMPIO);
  api.templateList.mockResolvedValue([PLANTILLA]);
  api.importCode.mockResolvedValue({ document: secuencia(), diagnostics: [] });
  api.emitCode.mockResolvedValue("@startuml\nnav -> svc : credenciales\n@enduml");
  api.recentList.mockResolvedValue([]);
  api.autosaveRead.mockResolvedValue(null);
  api.autosaveWrite.mockResolvedValue(undefined);
  api.appInfo.mockResolvedValue({
    name: "diagrammaker",
    version: "0.1.0",
    schemaVersion: 1,
    fileExtension: "dgm",
  });

  useDocumentStore.setState({ history: null, path: null, dirty: false, imported: [] });
  useCodePanel.setState({ open: false });
  useCommandPalette.setState({ open: false });
  useDiagnostics.setState({ open: false });
  useDialogs.setState({ dialog: null });
});

afterEach(cleanup);

/** Monta la aplicación y espera a que tenga documento. */
async function arrancar() {
  render(<App />);
  await waitFor(() => expect(useDocumentStore.getState().history).not.toBeNull());
}

/** Abre el único ejemplo y espera a que el documento sea el suyo. */
async function abrirEjemplo(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole("button", { name: "Ejemplos" }));
  await user.click(await screen.findByRole("button", { name: /inicio de sesión/i }));
  await waitFor(() => {
    expect(useDocumentStore.getState().history?.present.id).toBe("login");
  });
}

describe("la aplicación entera", () => {
  it("arranca con un lienzo en blanco", async () => {
    await arrancar();

    expect(api.documentNew).toHaveBeenCalledWith("architecture", expect.any(Number));
    expect(screen.getByRole("button", { name: "Ejemplos" })).toBeInTheDocument();
  });

  it("elige el lienzo que toca segun el tipo de diagrama", async () => {
    const user = userEvent.setup();
    await arrancar();

    // Una arquitectura va al motor de grafo...
    expect(screen.getByTestId("lienzo-de-grafo")).toBeInTheDocument();

    await abrirEjemplo(user);

    // ...y una secuencia al de carriles, que no es React Flow.
    await waitFor(() => {
      expect(screen.queryByTestId("lienzo-de-grafo")).not.toBeInTheDocument();
    });
    expect(screen.getByText("Navegador")).toBeInTheDocument();
  });

  it("abrir un ejemplo trae el diagrama y lo dibuja", async () => {
    const user = userEvent.setup();
    await arrancar();

    await abrirEjemplo(user);

    // El lienzo de carriles pinta los participantes del guion.
    expect(await screen.findByText("Navegador")).toBeInTheDocument();
    expect(screen.getByText("Servicio")).toBeInTheDocument();
    expect(screen.getByText("credenciales")).toBeInTheDocument();
  });

  it("la barra de estado cuenta lo que hay en el documento", async () => {
    const user = userEvent.setup();
    await arrancar();
    await abrirEjemplo(user);

    expect(await screen.findByText(/2 nodos/)).toBeInTheDocument();
  });

  it("el panel de código enseña el del diagrama abierto", async () => {
    const user = userEvent.setup();
    await arrancar();
    await abrirEjemplo(user);

    await user.click(screen.getByRole("button", { name: "Código" }));

    await waitFor(() => {
      expect(api.emitCode).toHaveBeenCalledWith("mermaid", expect.anything());
    });

    // `@startuml` solo esta en el panel; «credenciales» sale tambien en el
    // lienzo y no distinguiria uno de otro.
    expect(await screen.findByText(/@startuml/)).toBeInTheDocument();
  });

  it("los problemas del documento llegan al panel y seleccionan su culpable", async () => {
    const user = userEvent.setup();
    api.documentValidate.mockResolvedValue({
      ok: false,
      diagnostics: [
        {
          severity: "error",
          code: "unknownParticipant",
          message: "El mensaje `credenciales` tiene un destino que ya no existe.",
          target: "svc",
        },
      ],
    });
    await arrancar();
    await abrirEjemplo(user);

    // El contador aparece solo, al revalidarse el documento.
    const contador = await screen.findByRole("button", { name: /1 problema/ });
    await user.click(contador);

    const panel = screen.getByLabelText("Problemas del diagrama");
    await user.click(within(panel).getByRole("button", { name: /ya no existe/ }));

    expect(useDocumentStore.getState().selection).toEqual(["svc"]);
  });

  it("la paleta de comandos abre lo mismo que la barra", async () => {
    const user = userEvent.setup();
    await arrancar();

    await user.keyboard("{Control>}k{/Control}");
    await user.keyboard("exportar");
    await user.keyboard("{Enter}");

    expect(useDialogs.getState().dialog).toBe("export");
    expect(
      await screen.findByRole("heading", { name: "Exportar" }),
    ).toBeInTheDocument();
  });

  it("deshacer vuelve al diagrama anterior", async () => {
    const user = userEvent.setup();
    await arrancar();
    await abrirEjemplo(user);

    // Abrir un ejemplo descarta el historial a proposito: deshacer hasta
    // antes volveria a un documento que ya no tiene nada que ver.
    expect(useDocumentStore.getState().history?.past).toHaveLength(0);

    useDocumentStore.getState().edit("titulo", (d) => {
      d.meta.title = "Otro";
    });
    await waitFor(() => {
      expect(useDocumentStore.getState().history?.present.meta.title).toBe("Otro");
    });

    await user.keyboard("{Control>}z{/Control}");

    expect(useDocumentStore.getState().history?.present.meta.title).toBe(
      "Inicio de sesión",
    );
  });

  it("guarda una copia de recuperación mientras se trabaja", async () => {
    const user = userEvent.setup();
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      await arrancar();
      await abrirEjemplo(user);

      await vi.advanceTimersByTimeAsync(11_000);

      expect(api.autosaveWrite).toHaveBeenCalled();
    } finally {
      vi.useRealTimers();
    }
  });
});
