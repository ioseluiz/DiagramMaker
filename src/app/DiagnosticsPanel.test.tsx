import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import type { Diagnostic } from "@/model/generated/Diagnostic";
import type { Document } from "@/model/generated/Document";
import { initHistory } from "@/store/history";
import { useDocumentStore } from "@/store/documentStore";
import { DiagnosticsPanel } from "./DiagnosticsPanel";
import { useDiagnostics } from "./useDiagnostics";

function doc(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "activity",
    meta: { title: "prueba", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "manual",
    direction: "right",
  };
}

function problema(
  code: string,
  message: string,
  extra: Partial<Diagnostic> = {},
): Diagnostic {
  return { severity: "error", code, message, ...extra };
}

function aviso(
  code: string,
  message: string,
  extra: Partial<Diagnostic> = {},
): Diagnostic {
  return { severity: "warning", code, message, ...extra };
}

/** Deja el store con lo que diria el validador y lo que dijo la importacion. */
function state(current: Diagnostic[], imported: Diagnostic[] = []) {
  useDocumentStore.setState({
    history: initHistory(doc()),
    report: { ok: current.every((d) => d.severity !== "error"), diagnostics: current },
    imported,
    selection: [],
  });
}

/** Los textos de la lista que sigue a un encabezado de grupo. */
function grupo(title: string): string[] {
  const head = screen.getByRole("heading", { name: title });
  const list = head.nextElementSibling as HTMLElement;
  return within(list)
    .getAllByRole("listitem")
    .map((li) => li.textContent ?? "");
}

beforeEach(() => {
  useDiagnostics.setState({ open: true });
  state([]);
});

afterEach(cleanup);

describe("DiagnosticsPanel", () => {
  it("lo dice cuando no hay nada que senalar", () => {
    render(<DiagnosticsPanel />);

    expect(screen.getByText(/nada que señalar/i)).toBeInTheDocument();
  });

  it("lista los problemas del diagrama", () => {
    state([problema("manyStarts", "Hay dos arranques.")]);
    render(<DiagnosticsPanel />);

    expect(grupo("Del diagrama")).toHaveLength(1);
    expect(grupo("Del diagrama")[0]).toContain("Hay dos arranques.");
  });

  it("pone los errores por delante de los avisos", () => {
    state([
      aviso("danglingStep", "Un paso se queda colgando."),
      problema("manyStarts", "Hay dos arranques."),
    ]);
    render(<DiagnosticsPanel />);

    const filas = grupo("Del diagrama");
    expect(filas[0]).toContain("arranques");
    expect(filas[1]).toContain("colgando");
  });

  it("resume cuantos hay de cada clase", () => {
    state([problema("a", "uno"), problema("b", "dos"), aviso("c", "tres")]);
    render(<DiagnosticsPanel />);

    expect(screen.getByText("2 problemas · 1 aviso")).toBeInTheDocument();
  });

  it("ensena el codigo de cada diagnostico", () => {
    // Es lo que permite buscar ayuda sin depender del texto del mensaje.
    state([problema("manyStarts", "Hay dos arranques.")]);
    render(<DiagnosticsPanel />);

    expect(screen.getByText("manyStarts")).toBeInTheDocument();
  });

  it("pulsar un diagnostico selecciona lo que lo provoca", async () => {
    const user = userEvent.setup();
    state([problema("manyStarts", "Hay dos arranques.", { target: "n7" })]);
    render(<DiagnosticsPanel />);

    await user.click(screen.getByRole("button", { name: /dos arranques/i }));

    expect(useDocumentStore.getState().selection).toEqual(["n7"]);
  });

  it("un diagnostico sin culpable no se puede pulsar", () => {
    state([aviso("noStart", "No hay arranque.")]);
    render(<DiagnosticsPanel />);

    expect(screen.queryByRole("button", { name: /no hay arranque/i })).toBeNull();
    expect(screen.getByText("No hay arranque.")).toBeInTheDocument();
  });

  /* --- Lo de la importacion ---------------------------------------------- */

  it("separa lo que dijo la importacion de lo que dice el diagrama", () => {
    state(
      [aviso("danglingStep", "Un paso se queda colgando.")],
      [aviso("ignoredDirective", "La directiva se importa sin efecto.")],
    );
    render(<DiagnosticsPanel />);

    expect(grupo("Del diagrama")[0]).toContain("colgando");
    expect(grupo("De la importación")[0]).toContain("directiva");
  });

  it("lo de la importacion ensena la linea del codigo", () => {
    state(
      [],
      [aviso("ignoredDirective", "Sin efecto.", { span: { line: 7, column: 3 } })],
    );
    render(<DiagnosticsPanel />);

    expect(grupo("De la importación")[0]).toContain("línea 7:3");
  });

  it("lo de la importacion no selecciona nada: ya no hay a que apuntar", () => {
    // Un diagnostico de importacion habla del codigo, no del diagrama; su
    // `target`, si lo trae, es de un documento que ya se sustituyo.
    state([], [aviso("x", "Algo del codigo.", { target: "n1" })]);
    render(<DiagnosticsPanel />);

    expect(screen.queryByRole("button", { name: /algo del codigo/i })).toBeNull();
  });

  it("el grupo que no tiene nada no sale", () => {
    state([problema("a", "uno")]);
    render(<DiagnosticsPanel />);

    expect(screen.queryByRole("heading", { name: "De la importación" })).toBeNull();
  });

  it("se cierra", async () => {
    const user = userEvent.setup();
    render(<DiagnosticsPanel />);

    await user.click(screen.getByLabelText("Cerrar el panel de problemas"));

    expect(useDiagnostics.getState().open).toBe(false);
  });
});
