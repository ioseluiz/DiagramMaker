import { describe, expect, it } from "vitest";

import {
  ALPHA,
  SWATCHES,
  parseFill,
  parseHex,
  textColorOn,
  toHex,
  type Fill,
} from "./palette";

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

describe("parseFill", () => {
  it("lee lo que escribe esta paleta", () => {
    expect(parseFill("rgba(37, 99, 235, 0.16)")).toEqual({
      r: 37,
      g: 99,
      b: 235,
      alpha: 0.16,
    });
  });

  it("lee tambien hex y la sintaxis moderna de un documento viejo", () => {
    // El `.dgm` guardado antes del arreglo lleva dentro `rgb(r g b / a)`.
    expect(parseFill("#2563eb")).toEqual({ r: 37, g: 99, b: 235, alpha: 1 });
    expect(parseFill("#28f")).toEqual({ r: 34, g: 136, b: 255, alpha: 1 });
    expect(parseFill("rgb(37 99 235 / 0.16)")).toEqual({
      r: 37,
      g: 99,
      b: 235,
      alpha: 0.16,
    });
  });

  it("devuelve null en vez de inventarse un color", () => {
    // El selector se queda entonces en su valor de arranque, que es mejor
    // que mostrar negro y hacer creer que el nodo es negro.
    for (const malo of ["", null, undefined, "url(#grad)", "steelblue", "rgb(1 2)"]) {
      expect(parseFill(malo)).toBeNull();
    }
  });

  it("ida y vuelta por hex", () => {
    for (const hex of ["#000000", "#2563eb", "#ffffff", "#0d9488"]) {
      expect(toHex(parseHex(hex)!)).toBe(hex);
    }
  });
});

describe("textColorOn", () => {
  it("un tinte palido deja mandar al tema", () => {
    // Es lo que hacia el selector cuando solo habia tintes, y tiene que
    // seguir pasando: un 16 % se compone casi del todo con el fondo, que
    // cambia con el tema.
    for (const swatch of SWATCHES) {
      if (!swatch.value) continue;
      expect(textColorOn(parseFill(swatch.value)!)).toBeNull();
    }
  });

  it("un relleno solido oscuro pide texto claro, y al reves", () => {
    expect(textColorOn(parseFill("#1e3a8a")!)).toBe("#ffffff");
    expect(textColorOn(parseFill("#fde68a")!)).toBe("#16191d");
  });

  it("si fija un color, se lee mejor que el del tema. Siempre", () => {
    // La garantia entera, y la razon de que esta funcion exista. Barre todo
    // el cubo de color por los dos temas: si en alguna combinacion fijar el
    // color empeorara lo que el tema ya daba, sale aqui.
    for (let r = 0; r <= 255; r += 51) {
      for (let g = 0; g <= 255; g += 51) {
        for (let b = 0; b <= 255; b += 51) {
          for (const alpha of [0.05, 0.16, 0.3, 0.5, 0.7, 0.85, 1]) {
            const fill = { r, g, b, alpha };
            const texto = textColorOn(fill);
            if (!texto) continue;

            const fijado = Math.min(
              contraste(texto, fill, 255),
              contraste(texto, fill, 0),
            );
            const tema = Math.min(
              contraste("#16191d", fill, 255),
              contraste("#ffffff", fill, 0),
            );

            expect(fijado).toBeGreaterThan(tema);
          }
        }
      }
    }
  });

  it("y con un relleno opaco si lo fija, porque ahi el tema es una moneda al aire", () => {
    // Opaco el fondo no influye, asi que uno de los dos textos se lee mejor
    // que el otro y el tema puede acertar o no. Fijarlo quita el azar.
    //
    // Los tres ultimos son el motivo de que la regla no sea un umbral: con
    // ninguno de los dos textos pasan de 4,5 a 1, asi que un umbral los
    // habria devuelto al tema, que ahi acierta la mitad de las veces.
    for (const hex of [
      "#1e3a8a",
      "#fde68a",
      "#2563eb",
      "#64748b",
      "#0d9488",
      "#0077dd",
      "#008866",
      "#008888",
    ]) {
      expect(textColorOn(parseFill(hex)!)).not.toBeNull();
    }
  });
});

/** Contraste real entre un texto y un relleno compuesto sobre un fondo. */
function contraste(texto: string, fill: Fill, fondo: number): number {
  const mezcla = (c: number) => c * fill.alpha + fondo * (1 - fill.alpha);
  const lum = ({ r, g, b }: Fill) => {
    const canal = (v: number) => {
      const s = v / 255;
      return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * canal(r) + 0.7152 * canal(g) + 0.0722 * canal(b);
  };

  const a = lum(parseHex(texto)!);
  const b = lum({ r: mezcla(fill.r), g: mezcla(fill.g), b: mezcla(fill.b), alpha: 1 });
  const [alta, baja] = a > b ? [a, b] : [b, a];
  return (alta + 0.05) / (baja + 0.05);
}
