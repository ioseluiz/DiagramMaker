import { useEffect, useRef, useState } from "react";

import { useDocumentStore } from "@/store/documentStore";

/** Menu desplegable de archivos recientes. */
export function RecentMenu() {
  const recents = useDocumentStore((s) => s.recents);
  const forget = useDocumentStore((s) => s.forgetRecent);
  const open = useDocumentStore((s) => s.open);

  const [showing, setShowing] = useState(false);
  const wrapper = useRef<HTMLDivElement>(null);

  // Cerrar al pulsar fuera o con Escape, como cualquier menu del sistema.
  useEffect(() => {
    if (!showing) return;

    function onPointerDown(event: PointerEvent) {
      if (!wrapper.current?.contains(event.target as globalThis.Node)) {
        setShowing(false);
      }
    }
    function onKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") setShowing(false);
    }

    window.addEventListener("pointerdown", onPointerDown);
    window.addEventListener("keydown", onKeyDown);
    return () => {
      window.removeEventListener("pointerdown", onPointerDown);
      window.removeEventListener("keydown", onKeyDown);
    };
  }, [showing]);

  return (
    <div className="recent-menu" ref={wrapper}>
      <button
        type="button"
        onClick={() => setShowing((v) => !v)}
        disabled={recents.length === 0}
        aria-expanded={showing}
        title="Archivos recientes"
      >
        Recientes
      </button>

      {showing && (
        <ul className="recent-menu__list">
          {recents.map((file) => (
            <li key={file.path}>
              <button
                type="button"
                className="recent-menu__item"
                disabled={!file.exists}
                title={file.exists ? file.path : `No encontrado: ${file.path}`}
                onClick={() => {
                  setShowing(false);
                  void open(file.path);
                }}
              >
                <span className="recent-menu__name">{file.name}</span>
                <span className="recent-menu__path">{file.path}</span>
              </button>
              <button
                type="button"
                className="recent-menu__forget"
                title="Quitar de la lista"
                onClick={() => void forget(file.path)}
              >
                ×
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
