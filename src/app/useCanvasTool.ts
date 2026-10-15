import { useEffect } from "react";
import { create } from "zustand";

/**
 * Herramienta activa del lienzo.
 *
 * Vive en su propio store y no en el del documento porque no es parte del
 * diagrama: no se guarda, no entra en el historial de deshacer y no viaja al
 * exportar.
 *
 * Existe porque el arrastre izquierdo solo puede hacer una cosa. Con el
 * puntero selecciona por rectangulo; con la mano desplaza. Es el reparto de
 * draw.io, Figma y Excalidraw, incluidos los atajos.
 */
export type CanvasTool = "select" | "hand";

interface ToolState {
  tool: CanvasTool;
  setTool: (tool: CanvasTool) => void;
}

export const useCanvasTool = create<ToolState>()((set) => ({
  tool: "select",
  setTool: (tool) => set({ tool }),
}));

const SHORTCUTS: Record<string, CanvasTool> = { v: "select", h: "hand" };

/** Atajos `V` y `H`, los mismos que usan las demas herramientas de dibujo. */
export function useCanvasToolShortcuts(): void {
  const setTool = useCanvasTool((s) => s.setTool);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      // Sin modificadores: `Ctrl+V` es pegar, no cambiar de herramienta.
      if (event.ctrlKey || event.metaKey || event.altKey) return;

      const target = event.target as HTMLElement | null;
      if (
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target instanceof HTMLSelectElement ||
        target?.isContentEditable === true
      ) {
        return;
      }

      const tool = SHORTCUTS[event.key.toLowerCase()];
      if (tool) setTool(tool);
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [setTool]);
}
