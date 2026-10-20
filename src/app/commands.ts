import { useMemo } from "react";

import { SHAPE_LABEL } from "@/model/shapes";
import {
  selectCanRedo,
  selectCanUndo,
  selectDocument,
  useDocumentStore,
} from "@/store/documentStore";
import { useTheme } from "@/theme/useTheme";
import { useAddShape } from "./useAddShape";
import { useCanvasTool } from "./useCanvasTool";
import { useCodePanel } from "./useCodePanel";
import { useDiagnostics } from "./useDiagnostics";
import { useDialogs } from "./useDialogs";
import { useDocumentCommands } from "./useDocumentCommands";

/**
 * Todo lo que la aplicacion sabe hacer, en una sola lista.
 *
 * Es el registro del que beben la paleta de comandos y los atajos de teclado.
 * Tenerlo en un sitio es lo que evita el problema de siempre: que un atajo
 * exista pero no aparezca en ningun menu, o que un boton y su atajo hagan
 * cosas que se van separando.
 *
 * Lo que **no** entra aqui son las acciones que solo tienen sentido con algo
 * seleccionado en el lienzo —borrar, alinear— porque dependen del foco y no
 * del documento. Esas siguen donde estan.
 */
export interface Command {
  id: string;
  title: string;
  group: CommandGroup;
  /** Atajo que se muestra y que lo dispara. */
  shortcut?: string;
  /** Segundo atajo que tambien lo dispara, sin mostrarse. */
  altShortcut?: string;
  /**
   * Si el atajo vale tambien dentro de un campo de texto. Por defecto no:
   * ahi mandan los atajos del propio campo. Guardar es la excepcion de
   * siempre.
   */
  whileEditing?: boolean;
  /** `false` lo deja a la vista pero sin poder ejecutarse. */
  enabled: boolean;
  run: () => void;
}

export type CommandGroup = "Archivo" | "Edición" | "Ver" | "Herramientas" | "Añadir";

/* --- Atajos ----------------------------------------------------------------- */

export interface Shortcut {
  key: string;
  ctrl: boolean;
  shift: boolean;
  alt: boolean;
}

/**
 * Lee un atajo escrito como `Ctrl+Shift+Z`.
 *
 * Se declara como texto porque asi la lista de comandos se lee de un vistazo;
 * de convertirlo se encarga esto, que es lo unico que hay que probar.
 */
export function parseShortcut(text: string): Shortcut {
  const parts = text.split("+").map((p) => p.trim().toLowerCase());
  const key = parts[parts.length - 1] ?? "";

  return {
    key,
    ctrl: parts.includes("ctrl"),
    shift: parts.includes("shift"),
    alt: parts.includes("alt"),
  };
}

/** Como se escribe un atajo en pantalla. */
export function formatShortcut(text: string): string {
  return text
    .split("+")
    .map((part) => {
      const clean = part.trim();
      return clean.charAt(0).toUpperCase() + clean.slice(1).toLowerCase();
    })
    .join("+");
}

/** `true` si la pulsacion es ese atajo. */
export function matchesShortcut(
  event: Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey">,
  text: string,
): boolean {
  const shortcut = parseShortcut(text);

  return (
    event.key.toLowerCase() === shortcut.key &&
    event.ctrlKey === shortcut.ctrl &&
    event.shiftKey === shortcut.shift &&
    event.altKey === shortcut.alt
  );
}

/* --- Busqueda ---------------------------------------------------------------- */

/**
 * Deja el texto comparable: sin mayusculas y sin tildes.
 *
 * Sin quitar las tildes, buscar "codigo" no encontraria "Código", que es
 * justo lo que uno escribe con prisa.
 */
function plain(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase();
}

/**
 * Filtra por palabras sueltas y en cualquier orden: `exp svg` encuentra
 * "Exportar a SVG".
 */
export function filterCommands(commands: Command[], query: string): Command[] {
  const words = plain(query).split(/\s+/).filter(Boolean);
  if (words.length === 0) return commands;

  return commands.filter((command) => {
    const haystack = plain(`${command.group} ${command.title}`);
    return words.every((word) => haystack.includes(word));
  });
}

/* --- El registro -------------------------------------------------------------- */

export function useCommands(): Command[] {
  const doc = useDocumentStore(selectDocument);
  const busy = useDocumentStore((s) => s.busy);
  const canUndo = useDocumentStore(selectCanUndo);
  const canRedo = useDocumentStore(selectCanRedo);
  const undo = useDocumentStore((s) => s.undo);
  const redo = useDocumentStore((s) => s.redo);

  const { createNew, openFile, saveFile, saveFileAs } = useDocumentCommands();
  const { setPreference } = useTheme();
  const setTool = useCanvasTool((s) => s.setTool);
  const toggleCode = useCodePanel((s) => s.toggle);
  const showDialog = useDialogs((s) => s.show);
  const toggleProblems = useDiagnostics((s) => s.toggle);
  const { palette, addShape } = useAddShape();

  return useMemo(() => {
    const open = Boolean(doc);

    const commands: Command[] = [
      {
        id: "file.new",
        title: "Nuevo diagrama",
        group: "Archivo",
        shortcut: "Ctrl+N",
        enabled: true,
        run: () => createNew(),
      },
      {
        id: "file.open",
        title: "Abrir…",
        group: "Archivo",
        shortcut: "Ctrl+O",
        enabled: true,
        run: () => void openFile(),
      },
      {
        id: "file.save",
        title: "Guardar",
        group: "Archivo",
        shortcut: "Ctrl+S",
        whileEditing: true,
        enabled: open && !busy,
        run: () => void saveFile(),
      },
      {
        id: "file.saveAs",
        title: "Guardar como…",
        group: "Archivo",
        shortcut: "Ctrl+Shift+S",
        whileEditing: true,
        enabled: open && !busy,
        run: () => void saveFileAs(),
      },
      {
        id: "file.import",
        title: "Importar desde código",
        group: "Archivo",
        enabled: true,
        run: () => showDialog("import"),
      },
      {
        id: "file.export",
        title: "Exportar a SVG, PNG o PDF",
        group: "Archivo",
        shortcut: "Ctrl+E",
        enabled: open,
        run: () => showDialog("export"),
      },

      {
        id: "edit.undo",
        title: "Deshacer",
        group: "Edición",
        shortcut: "Ctrl+Z",
        enabled: canUndo,
        run: undo,
      },
      {
        id: "edit.redo",
        title: "Rehacer",
        group: "Edición",
        shortcut: "Ctrl+Shift+Z",
        altShortcut: "Ctrl+Y",
        enabled: canRedo,
        run: redo,
      },

      {
        id: "view.code",
        title: "Panel de código",
        group: "Ver",
        enabled: open,
        run: toggleCode,
      },
      {
        id: "view.problems",
        title: "Panel de problemas",
        group: "Ver",
        enabled: open,
        run: toggleProblems,
      },
      {
        id: "view.theme.light",
        title: "Tema claro",
        group: "Ver",
        enabled: true,
        run: () => setPreference("light"),
      },
      {
        id: "view.theme.dark",
        title: "Tema oscuro",
        group: "Ver",
        enabled: true,
        run: () => setPreference("dark"),
      },
      {
        id: "view.theme.system",
        title: "Tema del sistema",
        group: "Ver",
        enabled: true,
        run: () => setPreference("system"),
      },

      {
        id: "tool.select",
        title: "Herramienta de selección",
        group: "Herramientas",
        shortcut: "V",
        enabled: true,
        run: () => setTool("select"),
      },
      {
        id: "tool.hand",
        title: "Herramienta de mano",
        group: "Herramientas",
        shortcut: "H",
        enabled: true,
        run: () => setTool("hand"),
      },
    ];

    // Las formas dependen del tipo de diagrama, asi que la lista se arma con
    // la paleta que toque en vez de enumerarlas todas.
    for (const shape of palette) {
      commands.push({
        id: `add.${shape}`,
        title: `Añadir ${SHAPE_LABEL[shape].toLowerCase()}`,
        group: "Añadir",
        enabled: open,
        run: () => addShape(shape),
      });
    }

    return commands;
  }, [
    addShape,
    busy,
    canRedo,
    canUndo,
    createNew,
    doc,
    openFile,
    palette,
    redo,
    saveFile,
    saveFileAs,
    setPreference,
    setTool,
    showDialog,
    toggleCode,
    toggleProblems,
    undo,
  ]);
}
