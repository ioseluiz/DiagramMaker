import { useEffect, useMemo, useRef, useState } from "react";

import {
  filterCommands,
  formatShortcut,
  matchesShortcut,
  useCommands,
  type Command,
} from "./commands";
import { useCommandPalette } from "./useCommandPalette";
import "./CommandPalette.css";

/** Con que se abre. El segundo es el de VS Code, que mucha gente trae en los dedos. */
const OPEN_SHORTCUTS = ["Ctrl+K", "Ctrl+Shift+P"];

/**
 * Paleta de comandos.
 *
 * Es la otra puerta a todo lo que la aplicacion sabe hacer: lo mismo que la
 * barra de herramientas, pero escribiendo y sin tener que saber donde esta
 * cada boton. Lee del mismo registro que los atajos, asi que no puede
 * ofrecer algo que no exista ni olvidarse de algo nuevo.
 */
export function CommandPalette() {
  const open = useCommandPalette((s) => s.open);
  const toggle = useCommandPalette((s) => s.toggle);
  const close = useCommandPalette((s) => s.close);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      // Se abre incluso desde un campo de texto: es la via de escape cuando
      // uno no sabe donde esta.
      if (OPEN_SHORTCUTS.some((s) => matchesShortcut(event, s))) {
        event.preventDefault();
        toggle();
      }
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [toggle]);

  if (!open) return null;

  return <Palette onClose={close} />;
}

/**
 * El contenido va aparte para que se monte de cero cada vez: asi la busqueda
 * y la fila elegida empiezan limpias sin tener que vaciarlas a mano.
 */
function Palette({ onClose }: { onClose: () => void }) {
  const commands = useCommands();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const listRef = useRef<HTMLUListElement>(null);

  const found = useMemo(() => filterCommands(commands, query), [commands, query]);

  // Al escribir, la fila elegida vuelve arriba: dejarla donde estaba
  // apuntaria a un comando que ya no se ve.
  const index = Math.min(active, Math.max(found.length - 1, 0));

  useEffect(() => {
    listRef.current
      ?.querySelector('[aria-selected="true"]')
      ?.scrollIntoView({ block: "nearest" });
  }, [index]);

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }

    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive(found.length === 0 ? 0 : (index + 1) % found.length);
      return;
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive(found.length === 0 ? 0 : (index - 1 + found.length) % found.length);
      return;
    }

    if (event.key === "Enter") {
      event.preventDefault();
      run(found[index]);
    }
  }

  function run(command: Command | undefined) {
    if (!command?.enabled) return;
    // Primero se cierra: un comando que abre otro dialogo no debe quedar
    // debajo de la paleta.
    onClose();
    command.run();
  }

  return (
    <div className="modal-backdrop command-palette__backdrop" onPointerDown={onClose}>
      <div
        className="command-palette"
        role="dialog"
        aria-modal="true"
        aria-label="Paleta de comandos"
        onPointerDown={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <input
          className="command-palette__input"
          type="text"
          autoFocus
          value={query}
          placeholder="Escribe un comando…"
          aria-label="Buscar un comando"
          aria-controls="command-palette-list"
          onChange={(e) => {
            setQuery(e.target.value);
            setActive(0);
          }}
        />

        {found.length === 0 ? (
          <p className="command-palette__empty">No hay ningún comando que encaje.</p>
        ) : (
          <ul
            className="command-palette__list"
            id="command-palette-list"
            role="listbox"
            ref={listRef}
          >
            {found.map((command, position) => (
              <li
                key={command.id}
                role="option"
                aria-selected={position === index}
                aria-disabled={!command.enabled || undefined}
                className="command-palette__item"
                onPointerDown={(e) => {
                  e.preventDefault();
                  run(command);
                }}
                onPointerEnter={() => setActive(position)}
              >
                <span className="command-palette__group">{command.group}</span>
                <span className="command-palette__title">{command.title}</span>
                {command.shortcut && (
                  <kbd className="command-palette__key">
                    {formatShortcut(command.shortcut)}
                  </kbd>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

/** Lo que hace falta saber fuera para anunciar como se abre. */
export const PALETTE_SHORTCUT = OPEN_SHORTCUTS[0]!;
