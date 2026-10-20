import { create } from "zustand";

/**
 * Si el panel de problemas esta abierto.
 *
 * Como el resto del estado de vista —la herramienta del lienzo, el panel de
 * codigo, la paleta— no se guarda, no entra en el historial de deshacer y no
 * viaja al exportar.
 */
interface DiagnosticsState {
  open: boolean;
  toggle: () => void;
  show: () => void;
  close: () => void;
}

export const useDiagnostics = create<DiagnosticsState>()((set) => ({
  open: false,
  toggle: () => set((s) => ({ open: !s.open })),
  show: () => set({ open: true }),
  close: () => set({ open: false }),
}));
