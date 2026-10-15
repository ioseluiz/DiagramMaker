import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import type { NodeShape } from "./generated/NodeShape";
import {
  ALL_SHAPES,
  isFilled,
  primitivesFor,
  tablePrimitivesFor,
  type Primitive,
} from "./shapeGeometry";

function readGolden<T>(name: string): T {
  return JSON.parse(
    readFileSync(resolve(process.cwd(), `src-tauri/tests/golden/${name}`), "utf8"),
  ) as T;
}

/**
 * Gemela de `src-tauri/tests/golden_shapes.rs`: ambas leen este mismo fichero.
 * Si una de las dos implementaciones de la geometria cambia sin la otra, una
 * de las dos pruebas falla.
 */
interface GoldenCase {
  shape: NodeShape;
  width: number;
  height: number;
  primitives: Primitive[];
}

interface TableCase {
  rows: number;
  width: number;
  height: number;
  primitives: Primitive[];
}

// Relativo a la raiz del proyecto: en jsdom, `import.meta.url` es una URL
// http servida por Vite y `readFileSync` no la entiende.
const golden = readGolden<GoldenCase[]>("shape-primitives.json");
const tableGolden = readGolden<TableCase[]>("table-primitives.json");

describe("shapeGeometry contra el fichero dorado de Rust", () => {
  it("el fichero cubre todas las formas de contorno fijo", () => {
    expect(new Set(golden.map((c) => c.shape)).size).toBe(ALL_SHAPES.length);
  });

  it.each(golden)("$shape a $width×$height", (testCase) => {
    expect(primitivesFor(testCase.shape, testCase.width, testCase.height)).toEqual(
      testCase.primitives,
    );
  });
});

describe("primitivesFor", () => {
  it("devuelve algo para cada forma", () => {
    for (const shape of ALL_SHAPES) {
      expect(primitivesFor(shape, 160, 60).length).toBeGreaterThan(0);
    }
  });

  it("cada forma tiene al menos un cuerpo relleno", () => {
    for (const shape of ALL_SHAPES) {
      expect(primitivesFor(shape, 160, 60).some(isFilled)).toBe(true);
    }
  });

  it("las lineas interiores no se rellenan", () => {
    const cylinder = primitivesFor("cylinder", 120, 90);

    expect(isFilled(cylinder[0]!)).toBe(true);
    expect(isFilled(cylinder[1]!)).toBe(false);
  });
});

describe("geometria de tabla contra el fichero dorado de Rust", () => {
  it("el fichero cubre tablas vacias, pequenas y grandes", () => {
    expect(tableGolden.map((c) => c.rows)).toEqual([0, 1, 4, 12]);
  });

  it.each(tableGolden)("tabla de $rows columnas", (testCase) => {
    expect(tablePrimitivesFor(testCase.rows, testCase.width, testCase.height)).toEqual(
      testCase.primitives,
    );
  });
});

describe("tablePrimitivesFor", () => {
  it("una tabla sin columnas es solo el marco", () => {
    expect(tablePrimitivesFor(0, 220, 28)).toHaveLength(1);
  });

  it("el marco es la unica primitiva rellena", () => {
    const p = tablePrimitivesFor(4, 220, 116);

    expect(p.filter(isFilled)).toHaveLength(1);
  });
});
