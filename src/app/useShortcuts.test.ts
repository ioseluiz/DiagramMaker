import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { act, renderHook } from "@testing-library/react";

import type { Document } from "@/model/generated/Document";
import { initHistory } from "@/store/history";
import { useDocumentStore } from "@/store/documentStore";
import { useCanvasTool } from "./useCanvasTool";
import { useDialogs } from "./useDialogs";
import { useShortcuts } from "./useShortcuts";

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

function press(key: string, init: KeyboardEventInit = {}) {
  window.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...init }));
}

/** Pulsa con el foco dentro de un campo de texto. */
function pressInside(
  tag: "input" | "textarea",
  key: string,
  init: KeyboardEventInit = {},
) {
  const field = document.createElement(tag);
  document.body.append(field);
  field.focus();
  field.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true, ...init }));
}

beforeEach(() => {
  useCanvasTool.setState({ tool: "select" });
  useDialogs.setState({ dialog: null });
  useDocumentStore.setState({ history: initHistory(doc()), path: null, dirty: false });
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("useShortcuts", () => {
  it("H activa la mano y V vuelve al puntero", () => {
    renderHook(() => useShortcuts());

    press("h");
    expect(useCanvasTool.getState().tool).toBe("hand");

    press("v");
    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("acepta mayusculas", () => {
    renderHook(() => useShortcuts());

    press("H");

    expect(useCanvasTool.getState().tool).toBe("hand");
  });

  it("Ctrl+V no cambia de herramienta: eso es pegar", () => {
    renderHook(() => useShortcuts());
    useCanvasTool.setState({ tool: "hand" });

    press("v", { ctrlKey: true });

    expect(useCanvasTool.getState().tool).toBe("hand");
  });

  it("escribir una `h` en un campo de texto no activa la mano", () => {
    renderHook(() => useShortcuts());

    pressInside("input", "h");

    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("tampoco en un area de texto", () => {
    renderHook(() => useShortcuts());

    pressInside("textarea", "h");

    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("otras teclas no hacen nada", () => {
    renderHook(() => useShortcuts());

    press("q");

    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("Ctrl+E abre el dialogo de exportar", () => {
    renderHook(() => useShortcuts());

    act(() => press("e", { ctrlKey: true }));

    expect(useDialogs.getState().dialog).toBe("export");
  });

  // Cambia el documento dejando que React se entere: el registro de comandos
  // se arma en el render, asi que sin `act` el oyente seguiria viendo el
  // deshacer apagado.
  function cambiar(title: string) {
    act(() => {
      useDocumentStore.getState().edit("prueba", (d) => {
        d.meta.title = title;
      });
    });
  }

  function titulo(): string | undefined {
    return useDocumentStore.getState().history?.present.meta.title;
  }

  it("Ctrl+Z deshace", () => {
    renderHook(() => useShortcuts());

    cambiar("cambiado");
    expect(titulo()).toBe("cambiado");

    act(() => press("z", { ctrlKey: true }));

    expect(titulo()).toBe("prueba");
  });

  it("Ctrl+Shift+Z rehace, y Ctrl+Y tambien", () => {
    renderHook(() => useShortcuts());
    cambiar("cambiado");

    // Cada rehacer se comprueba contra un deshacer que de verdad ocurrio; si
    // no, la prueba pasaria igual sin rehacer nada.
    act(() => press("z", { ctrlKey: true }));
    expect(titulo()).toBe("prueba");
    act(() => press("z", { ctrlKey: true, shiftKey: true }));
    expect(titulo()).toBe("cambiado");

    act(() => press("z", { ctrlKey: true }));
    expect(titulo()).toBe("prueba");
    act(() => press("y", { ctrlKey: true }));
    expect(titulo()).toBe("cambiado");
  });

  it("Ctrl+Z dentro de un campo de texto lo deja al campo", () => {
    renderHook(() => useShortcuts());
    cambiar("cambiado");

    act(() => pressInside("input", "z", { ctrlKey: true }));

    expect(titulo()).toBe("cambiado");
  });

  it("un comando apagado no se ejecuta pero se traga la pulsacion", () => {
    // Sin nada que deshacer, `Ctrl+Z` no debe caer en el deshacer del
    // navegador y borrar texto de otro sitio.
    renderHook(() => useShortcuts());

    const event = new KeyboardEvent("keydown", {
      key: "z",
      ctrlKey: true,
      bubbles: true,
      cancelable: true,
    });
    window.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
    expect(useDocumentStore.getState().history?.past).toHaveLength(0);
  });

  it("deja de escuchar al desmontarse", () => {
    const { unmount } = renderHook(() => useShortcuts());
    unmount();

    press("h");

    expect(useCanvasTool.getState().tool).toBe("select");
  });
});
