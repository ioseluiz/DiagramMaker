import { describe, expect, it } from "vitest";

import {
  filterCommands,
  formatShortcut,
  matchesShortcut,
  parseShortcut,
  type Command,
} from "./commands";

function command(title: string, group: Command["group"] = "Archivo"): Command {
  return { id: title, title, group, enabled: true, run: () => {} };
}

function key(
  k: string,
  init: { ctrlKey?: boolean; shiftKey?: boolean; altKey?: boolean } = {},
) {
  return {
    key: k,
    ctrlKey: false,
    metaKey: false,
    shiftKey: false,
    altKey: false,
    ...init,
  };
}

describe("parseShortcut", () => {
  it("separa la tecla de los modificadores", () => {
    expect(parseShortcut("Ctrl+Shift+Z")).toEqual({
      key: "z",
      ctrl: true,
      shift: true,
      alt: false,
    });
  });

  it("una tecla suelta no lleva modificadores", () => {
    expect(parseShortcut("V")).toEqual({
      key: "v",
      ctrl: false,
      shift: false,
      alt: false,
    });
  });
});

describe("formatShortcut", () => {
  it("escribe cada parte con la inicial en mayuscula", () => {
    expect(formatShortcut("ctrl+shift+z")).toBe("Ctrl+Shift+Z");
    expect(formatShortcut("v")).toBe("V");
  });
});

describe("matchesShortcut", () => {
  it("encaja la tecla y sus modificadores", () => {
    expect(matchesShortcut(key("s", { ctrlKey: true }), "Ctrl+S")).toBe(true);
  });

  it("no distingue mayusculas", () => {
    expect(matchesShortcut(key("S", { ctrlKey: true }), "Ctrl+S")).toBe(true);
  });

  it("un modificador de mas no encaja", () => {
    // Si no, `Ctrl+Shift+Z` dispararia tambien el deshacer.
    expect(matchesShortcut(key("z", { ctrlKey: true, shiftKey: true }), "Ctrl+Z")).toBe(
      false,
    );
  });

  it("un modificador de menos tampoco", () => {
    expect(matchesShortcut(key("z", { ctrlKey: true }), "Ctrl+Shift+Z")).toBe(false);
  });

  it("una tecla suelta no encaja con el modificador puesto", () => {
    // `Ctrl+V` es pegar, no cambiar de herramienta.
    expect(matchesShortcut(key("v", { ctrlKey: true }), "V")).toBe(false);
  });
});

describe("filterCommands", () => {
  const todos = [
    command("Guardar como…"),
    command("Exportar a SVG, PNG o PDF"),
    command("Panel de código", "Ver"),
    command("Deshacer", "Edición"),
  ];

  it("sin texto los devuelve todos", () => {
    expect(filterCommands(todos, "")).toHaveLength(4);
  });

  it("encuentra por un trozo del nombre", () => {
    expect(filterCommands(todos, "guard").map((c) => c.title)).toEqual([
      "Guardar como…",
    ]);
  });

  it("las palabras valen en cualquier orden", () => {
    expect(filterCommands(todos, "svg exp").map((c) => c.title)).toEqual([
      "Exportar a SVG, PNG o PDF",
    ]);
  });

  it("no distingue mayusculas", () => {
    expect(filterCommands(todos, "DESHACER")).toHaveLength(1);
  });

  it("no distingue tildes", () => {
    // Quien busca con prisa escribe "codigo", no "código".
    expect(filterCommands(todos, "codigo").map((c) => c.title)).toEqual([
      "Panel de código",
    ]);
  });

  it("tambien busca por el grupo", () => {
    expect(filterCommands(todos, "ver").map((c) => c.title)).toEqual([
      "Panel de código",
    ]);
  });

  it("lo que no encaja no sale", () => {
    expect(filterCommands(todos, "zzz")).toHaveLength(0);
  });
});
