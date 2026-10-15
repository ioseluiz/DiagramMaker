import { useEffect, useRef } from "react";

import "./ContextMenu.css";

/**
 * Menu contextual del lienzo.
 *
 * Se posiciona en coordenadas de pantalla y se recoloca solo si no cabe:
 * un menu que se sale por el borde derecho de la ventana es peor que no
 * tenerlo.
 */
export interface MenuItem {
  label: string;
  onSelect: () => void;
  /** Dibuja una linea encima para separar grupos. */
  separated?: boolean;
  danger?: boolean;
  disabled?: boolean;
}

export interface ContextMenuProps {
  x: number;
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

/** Tamano estimado para decidir si el menu cabe donde se pidio. */
const WIDTH = 190;
const ITEM_HEIGHT = 28;
const MARGIN = 8;

export function ContextMenu({ x, y, items, onClose }: ContextMenuProps) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function onPointerDown(event: PointerEvent) {
      if (!ref.current?.contains(event.target as globalThis.Node)) onClose();
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") onClose();
    }

    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    // Un desplazamiento del lienzo dejaria el menu apuntando a otro sitio.
    window.addEventListener("wheel", onClose, { passive: true });

    return () => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("wheel", onClose);
    };
  }, [onClose]);

  if (items.length === 0) return null;

  const height = items.length * ITEM_HEIGHT + MARGIN;
  const left = Math.min(x, window.innerWidth - WIDTH - MARGIN);
  const top = Math.min(y, window.innerHeight - height - MARGIN);

  return (
    <div
      ref={ref}
      className="context-menu"
      role="menu"
      style={{ left: Math.max(MARGIN, left), top: Math.max(MARGIN, top), width: WIDTH }}
    >
      {items.map((item, index) => (
        <button
          key={index}
          type="button"
          role="menuitem"
          className="context-menu__item"
          data-separated={item.separated || undefined}
          data-danger={item.danger || undefined}
          disabled={item.disabled}
          onClick={() => {
            item.onSelect();
            onClose();
          }}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
