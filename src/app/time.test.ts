import { describe, expect, it } from "vitest";

import { basename, relativeTime } from "./time";

const NOW = Date.UTC(2026, 9, 15, 12, 0, 0);
const SECOND = 1_000;
const MINUTE = 60 * SECOND;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

describe("relativeTime", () => {
  it("habla en espanol, no en el idioma del sistema", () => {
    // Regresion: con el locale del sistema salia «13 min. ago» dentro de un
    // aviso que dice «Cambios sin guardar».
    expect(relativeTime(NOW - 13 * MINUTE, NOW)).toMatch(/^hace /);
    expect(relativeTime(NOW - 3 * HOUR, NOW)).not.toMatch(/ago/);
  });

  it("describe el pasado reciente en segundos", () => {
    expect(relativeTime(NOW - 20 * SECOND, NOW)).toMatch(/20/);
  });

  it("pasa a minutos al superar el minuto", () => {
    const text = relativeTime(NOW - 12 * MINUTE, NOW);

    expect(text).toMatch(/12/);
    expect(text).not.toMatch(/720/);
  });

  it("pasa a horas al superar la hora", () => {
    const text = relativeTime(NOW - 3 * HOUR, NOW);

    expect(text).toMatch(/3/);
    expect(text).not.toMatch(/180/);
  });

  it("pasa a dias al superar el dia", () => {
    const text = relativeTime(NOW - 5 * DAY, NOW);

    expect(text).toMatch(/5/);
  });

  it("nunca devuelve una cadena vacia, por lejano que sea", () => {
    for (const ago of [0, SECOND, MINUTE, HOUR, DAY, 400 * DAY]) {
      expect(relativeTime(NOW - ago, NOW).length).toBeGreaterThan(0);
    }
  });

  it("el texto es corto: cabe en un aviso pequeno", () => {
    for (const ago of [30 * SECOND, 12 * MINUTE, 3 * HOUR, 5 * DAY]) {
      expect(relativeTime(NOW - ago, NOW).length).toBeLessThanOrEqual(20);
    }
  });

  it("tolera marcas de tiempo en el futuro sin romperse", () => {
    expect(relativeTime(NOW + 5 * MINUTE, NOW).length).toBeGreaterThan(0);
  });
});

describe("basename", () => {
  it("se queda con el nombre del archivo en rutas de Windows", () => {
    expect(basename("C:\\proyectos\\esquema.dgm")).toBe("esquema.dgm");
  });

  it("tambien con separadores de barra", () => {
    expect(basename("/home/jl/esquema.dgm")).toBe("esquema.dgm");
  });

  it("devuelve la entrada si no hay separadores", () => {
    expect(basename("esquema.dgm")).toBe("esquema.dgm");
  });

  it("no devuelve vacio con una ruta que acaba en separador", () => {
    expect(basename("C:\\proyectos\\")).toBe("C:\\proyectos\\");
  });
});
