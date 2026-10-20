import { create } from "zustand";

/**
 * Que dialogo modal esta abierto.
 *
 * Vivia dentro de la barra de herramientas, que era el unico sitio desde
 * donde se abrian. Ahora tambien los abre la paleta de comandos, asi que el
 * estado sube a donde los dos pueden verlo.
 *
 * Como el resto del estado de vista —la herramienta del lienzo, el panel de
 * codigo— no se guarda, no entra en el historial y no viaja al exportar.
 */
export type Dialog = "import" | "export" | "template";

interface DialogState {
  dialog: Dialog | null;
  show: (dialog: Dialog) => void;
  close: () => void;
}

export const useDialogs = create<DialogState>()((set) => ({
  dialog: null,
  show: (dialog) => set({ dialog }),
  close: () => set({ dialog: null }),
}));
