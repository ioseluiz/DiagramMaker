import { create } from "zustand";

/**
 * Visibilidad del panel de codigo.
 *
 * Igual que la herramienta del lienzo, es estado de vista: no se guarda, no
 * entra en el historial de deshacer y no viaja al exportar.
 */
interface CodePanelState {
  open: boolean;
  toggle: () => void;
  close: () => void;
}

export const useCodePanel = create<CodePanelState>()((set) => ({
  open: false,
  toggle: () => set((s) => ({ open: !s.open })),
  close: () => set({ open: false }),
}));
