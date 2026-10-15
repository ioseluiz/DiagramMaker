import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import type { NodeShape } from "./generated/NodeShape";
import { ALL_SHAPES, isFilled, primitivesFor, type Primitive } from "./shapeGeometry";

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

const golden: GoldenCase[] = JSON.parse(
  readFileSync(
    // Relativo a la raiz del proyecto: en jsdom, `import.meta.url` es una URL
    // http servida por Vite y `readFileSync` no la entiende.
    resolve(process.cwd(), "src-tauri/tests/golden/shape-primitives.json"),
    "utf8",
  ),
) as GoldenCase[];

describe("shapeGeometry contra el fichero dorado de Rust", () => {
  it("el fichero cubre las ocho formas", () => {
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
