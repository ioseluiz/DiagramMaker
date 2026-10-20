import { useEffect } from "react";

import { matchesShortcut, useCommands, type Command } from "./commands";

/**
 * `true` si lo que tiene el foco es un sitio donde se escribe.
 *
 * Ahi mandan los atajos del propio campo: `Ctrl+Z` deshace la ultima letra,
 * no la ultima operacion del diagrama.
 */
export function isEditing(target: EventTarget | null): boolean {
  const element = target as HTMLElement | null;

  return (
    element instanceof HTMLInputElement ||
    element instanceof HTMLTextAreaElement ||
    element instanceof HTMLSelectElement ||
    element?.isContentEditable === true
  );
}

/** El comando al que corresponde una pulsacion, si hay alguno. */
export function commandFor(
  commands: Command[],
  event: KeyboardEvent,
  editing: boolean,
): Command | undefined {
  return commands.find((command) => {
    if (editing && !command.whileEditing) return false;

    const hit =
      (command.shortcut && matchesShortcut(event, command.shortcut)) ||
      (command.altShortcut && matchesShortcut(event, command.altShortcut));

    return Boolean(hit);
  });
}

/**
 * Un solo oyente de teclado para toda la aplicacion, alimentado por el
 * registro de comandos.
 *
 * Antes habia dos —uno para los comandos de documento y otro para las
 * herramientas del lienzo— y cada uno decidia por su cuenta que hacer dentro
 * de un campo de texto. Con un solo sitio, lo que se ve en la paleta y lo que
 * responde al teclado no pueden separarse.
 */
export function useShortcuts(): void {
  const commands = useCommands();

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const command = commandFor(commands, event, isEditing(event.target));
      if (!command) return;

      // Un comando apagado se traga igualmente la pulsacion: que `Ctrl+Z` sin
      // nada que deshacer dispare el deshacer del navegador seria peor.
      event.preventDefault();
      if (command.enabled) command.run();
    }

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [commands]);
}
