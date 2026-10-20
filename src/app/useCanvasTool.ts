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
 * draw.io, Figma y Excalidraw, incluidos los atajos `V` y `H`, que declara
 * el registro de comandos.
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
