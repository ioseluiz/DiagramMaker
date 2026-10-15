import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { renderHook } from "@testing-library/react";

import { useCanvasTool, useCanvasToolShortcuts } from "./useCanvasTool";

function press(key: string, init: KeyboardEventInit = {}) {
  window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...init }));
}

beforeEach(() => {
  useCanvasTool.setState({ tool: "select" });
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("useCanvasTool", () => {
  it("arranca con el puntero", () => {
    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("H activa la mano y V vuelve al puntero", () => {
    renderHook(() => useCanvasToolShortcuts());

    press("h");
    expect(useCanvasTool.getState().tool).toBe("hand");

    press("v");
    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("acepta mayusculas", () => {
    renderHook(() => useCanvasToolShortcuts());

    press("H");

    expect(useCanvasTool.getState().tool).toBe("hand");
  });

  it("Ctrl+V no cambia de herramienta: eso es pegar", () => {
    renderHook(() => useCanvasToolShortcuts());
    useCanvasTool.setState({ tool: "hand" });

    press("v", { ctrlKey: true });

    expect(useCanvasTool.getState().tool).toBe("hand");
  });

  it("escribir una `h` en un campo de texto no activa la mano", () => {
    renderHook(() => useCanvasToolShortcuts());

    const input = document.createElement("input");
    document.body.append(input);
    input.focus();
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "h", bubbles: true }));

    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("tampoco en un area de texto", () => {
    renderHook(() => useCanvasToolShortcuts());

    const area = document.createElement("textarea");
    document.body.append(area);
    area.focus();
    area.dispatchEvent(new KeyboardEvent("keydown", { key: "h", bubbles: true }));

    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("otras teclas no hacen nada", () => {
    renderHook(() => useCanvasToolShortcuts());

    press("q");

    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("deja de escuchar al desmontarse", () => {
    const { unmount } = renderHook(() => useCanvasToolShortcuts());
    unmount();

    press("h");

    expect(useCanvasTool.getState().tool).toBe("select");
  });
});
