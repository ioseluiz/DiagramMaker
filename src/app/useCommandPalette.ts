import { create } from "zustand";

/**
 * Si la paleta de comandos esta abierta.
 *
 * Sale del propio componente para que tambien pueda abrirla la barra de
 * estado: un atajo que no esta escrito en ningun sitio no lo conoce nadie.
 */
interface PaletteState {
  open: boolean;
  toggle: () => void;
  close: () => void;
}

export const useCommandPalette = create<PaletteState>()((set) => ({
  open: false,
  toggle: () => set((s) => ({ open: !s.open })),
  close: () => set({ open: false }),
}));
