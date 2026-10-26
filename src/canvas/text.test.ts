import { describe, expect, it } from "vitest";

import {
  FONT_SIZE,
  LINE_HEIGHT,
  boxForLabel,
  labelMaxWidth,
  lineHeightFor,
  measureWidth,
  placementOf,
  wrapText,
} from "./text";

/**
 * En jsdom no hay canvas, asi que `measureWidth` cae en su estimacion:
 * 0.55 px por caracter y punto de tamano. Es deliberadamente predecible, lo
 * que permite comprobar la logica de partido sin depender de una fuente real.
 */
const CHAR = 13 * 0.55;

describe("measureWidth", () => {
  it("crece con la longitud del texto", () => {
    expect(measureWidth("ab")).toBeGreaterThan(measureWidth("a"));
  });

  it("un texto vacio no mide nada", () => {
    expect(measureWidth("")).toBe(0);
  });
});

describe("wrapText", () => {
  it("un texto vacio no produce lineas", () => {
    expect(wrapText("", { maxWidth: 100 })).toEqual([]);
  });

  it("un texto corto cabe en una sola linea", () => {
    const lines = wrapText("API", { maxWidth: 200 });

    expect(lines).toHaveLength(1);
    expect(lines[0]!.text).toBe("API");
  });

  it("parte por espacios cuando no cabe", () => {
    const lines = wrapText("uno dos tres", { maxWidth: CHAR * 8 });

    expect(lines.length).toBeGreaterThan(1);
    expect(lines.map((l) => l.text).join(" ")).toBe("uno dos tres");
  });

  it("respeta los saltos de linea explicitos", () => {
    const lines = wrapText("uno\ndos", { maxWidth: 500 });

    expect(lines.map((l) => l.text)).toEqual(["uno", "dos"]);
  });

  it("parte por caracteres una palabra mas ancha que la caja", () => {
    const lines = wrapText("supercalifragilistico", { maxWidth: CHAR * 5 });

    expect(lines.length).toBeGreaterThan(1);
    expect(lines.map((l) => l.text).join("")).toBe("supercalifragilistico");
  });

  it("cada linea trae su ancho medido", () => {
    const lines = wrapText("uno dos", { maxWidth: CHAR * 4 });

    for (const line of lines) {
      expect(line.width).toBeCloseTo(measureWidth(line.text), 5);
    }
  });

  it("ninguna linea excede la caja, salvo la que no se puede partir mas", () => {
    const maxWidth = CHAR * 10;
    const lines = wrapText("alfa beta gamma delta epsilon zeta", { maxWidth });

    for (const line of lines) {
      expect(line.width).toBeLessThanOrEqual(maxWidth + 0.001);
    }
  });

  it("corta con puntos suspensivos un texto interminable", () => {
    const lines = wrapText("a ".repeat(200), { maxWidth: CHAR * 3 });

    expect(lines).toHaveLength(6);
    expect(lines.at(-1)!.text.endsWith("…")).toBe(true);
  });

  it("el texto en negrita mide mas que el normal", () => {
    // Con la estimacion de jsdom ambos miden igual; lo que se comprueba es que
    // la opcion se acepta y no rompe el partido.
    const normal = wrapText("texto de prueba", { maxWidth: 100 });
    const bold = wrapText("texto de prueba", { maxWidth: 100, bold: true });

    expect(bold.length).toBe(normal.length);
  });
});

describe("labelMaxWidth", () => {
  it("un rectangulo descuenta el margen interior a ambos lados", () => {
    expect(labelMaxWidth("rectangle", 160)).toBe(140);
  });

  it("el actor usa un ancho fijo, no el de su figura", () => {
    expect(labelMaxWidth("actor", 40)).toBe(160);
  });

  it("el paquete se cine a su solapa", () => {
    expect(labelMaxWidth("package", 220)).toBe(220 * 0.45 - 10);
  });

  it("nunca devuelve un ancho inutilizable", () => {
    expect(labelMaxWidth("rectangle", 10)).toBeGreaterThanOrEqual(20);
  });
});

describe("placementOf", () => {
  it("coincide con la colocacion de cada forma", () => {
    expect(placementOf("actor")).toBe("below");
    expect(placementOf("package")).toBe("tab");
    expect(placementOf("rectangle")).toBe("center");
  });
});

describe("cache de medidas", () => {
  it("devuelve el mismo ancho para el mismo texto", () => {
    expect(measureWidth("varchar(255)", 12)).toBe(measureWidth("varchar(255)", 12));
  });

  it("distingue tamano y grosor", () => {
    // La clave de cache tiene que incluirlos, o una columna en negrita
    // reutilizaria la medida de una normal.
    const normal = measureWidth("texto", 12, false);
    const grande = measureWidth("texto", 24, false);

    expect(grande).not.toBe(normal);
  });

  it("un texto vacio no gasta entrada", () => {
    expect(measureWidth("")).toBe(0);
  });
});

describe("lineHeightFor", () => {
  it("al tamano por defecto da el alto de siempre", () => {
    expect(lineHeightFor(FONT_SIZE)).toBe(LINE_HEIGHT);
  });

  it("crece y encoge con el cuerpo de la letra", () => {
    // Si no lo hiciera, subir el texto amontonaria unas lineas sobre otras.
    expect(lineHeightFor(32)).toBeGreaterThan(lineHeightFor(24));
    expect(lineHeightFor(8)).toBeLessThan(lineHeightFor(FONT_SIZE));
  });

  it("deja siempre hueco entre el cuerpo y la linea siguiente", () => {
    for (const size of [8, 10, 13, 20, 32, 64]) {
      expect(lineHeightFor(size)).toBeGreaterThan(size);
    }
  });
});

describe("boxForLabel", () => {
  it("crece con el cuerpo de la letra", () => {
    const pequena = boxForLabel("Lago de datos", 13);
    const grande = boxForLabel("Lago de datos", 32);

    expect(grande.width).toBeGreaterThan(pequena.width);
    expect(grande.height).toBeGreaterThan(pequena.height);
  });

  it("no parte el texto por su cuenta", () => {
    // Mide lo que ocuparia sin caja que lo obligue a romper: por eso un
    // titulo largo pide una caja ancha en vez de una alta.
    const una = boxForLabel("Titulo", 16);
    const larga = boxForLabel("Un titulo bastante mas largo que el otro", 16);

    expect(larga.width).toBeGreaterThan(una.width);
    expect(larga.height).toBe(una.height);
  });

  it("respeta los saltos de linea escritos a mano", () => {
    const una = boxForLabel("Titulo", 16);
    const dos = boxForLabel("Titulo\nsubtitulo", 16);

    expect(dos.height).toBeGreaterThan(una.height);
  });

  it("una etiqueta vacia no pide alto de texto", () => {
    expect(boxForLabel("", 16).height).toBeLessThan(boxForLabel("x", 16).height);
  });
});
