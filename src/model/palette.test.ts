import { describe, expect, it } from "vitest";

import { ALPHA, SWATCHES } from "./palette";

describe("SWATCHES", () => {
  const conColor = SWATCHES.filter((s) => s.value !== null);

  it("guarda los tintes a la opacidad de los tintes", () => {
    for (const swatch of conColor) {
      expect(swatch.value).toContain(`, ${ALPHA})`);
    }
  });

  it("pinta el boton mas opaco que el relleno", () => {
    // A la opacidad del relleno la muestra seria casi invisible y el
    // selector parecia una fila de botones vacios. Paso de verdad.
    for (const swatch of conColor) {
      expect(alfaDe(swatch.preview)).toBeGreaterThan(ALPHA * 2);
    }
  });

  it("el boton es el mismo color que el relleno", () => {
    // Lo unico que puede cambiar entre los dos es la opacidad: si el boton
    // dijera un color y el nodo saliera de otro, el selector mentiria.
    for (const swatch of conColor) {
      expect(componentesDe(swatch.preview)).toEqual(componentesDe(swatch.value!));
    }
  });

  it("todos usan la sintaxis que entiende el exportador", () => {
    // `rgb(r g b / a)` el navegador la entiende y `resvg` la pinta negro.
    for (const swatch of conColor) {
      expect(swatch.value).toMatch(/^rgba\(\d+, \d+, \d+, [\d.]+\)$/);
      expect(swatch.preview).toMatch(/^rgba\(\d+, \d+, \d+, [\d.]+\)$/);
    }
  });

  it("ofrece quitar el color", () => {
    expect(SWATCHES.some((s) => s.value === null)).toBe(true);
  });
});

function partesDe(color: string): number[] {
  return color
    .replace(/^rgba\(|\)$/g, "")
    .split(",")
    .map((n) => Number(n.trim()));
}

function componentesDe(color: string): number[] {
  return partesDe(color).slice(0, 3);
}

function alfaDe(color: string): number {
  return partesDe(color)[3]!;
}
