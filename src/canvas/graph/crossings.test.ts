import { describe, expect, it } from "vitest";

import type { Point } from "@/model/generated/Point";
import { HOP_RADIUS, findHops, hopPath } from "./crossings";

const line = (...coords: [number, number][]): Point[] =>
  coords.map(([x, y]) => ({ x, y }));

/** Dos lineas que se cruzan en (50,50) formando una cruz. */
function cross(): Map<string, Point[]> {
  return new Map([
    ["a", line([0, 50], [100, 50])],
    ["b", line([50, 0], [50, 100])],
  ]);
}

describe("findHops", () => {
  it("una sola linea no salta", () => {
    const hops = findHops(new Map([["a", line([0, 0], [100, 100])]]), ["a"]);

    expect(hops.size).toBe(0);
  });

  it("dos lineas que se cruzan producen un salto", () => {
    const hops = findHops(cross(), ["a", "b"]);

    expect(hops.get("b")).toHaveLength(1);
    expect(hops.get("b")![0]).toMatchObject({ x: 50, y: 50 });
  });

  it("salta la de encima, no la de debajo", () => {
    // El orden decide quien pasa por encima; la primera no salta nunca.
    expect(findHops(cross(), ["a", "b"]).has("a")).toBe(false);
    expect(findHops(cross(), ["b", "a"]).has("b")).toBe(false);
    expect(findHops(cross(), ["b", "a"]).has("a")).toBe(true);
  });

  it("el resultado no depende del orden en que se recorran los pares", () => {
    const uno = findHops(cross(), ["a", "b"]);
    const dos = findHops(cross(), ["a", "b"]);

    expect(uno).toEqual(dos);
  });

  it("dos lineas que no se tocan no saltan", () => {
    const hops = findHops(
      new Map([
        ["a", line([0, 0], [100, 0])],
        ["b", line([0, 50], [100, 50])],
      ]),
      ["a", "b"],
    );

    expect(hops.size).toBe(0);
  });

  it("dos lineas paralelas superpuestas tampoco", () => {
    // Un puente ahi no aclararia nada.
    const hops = findHops(
      new Map([
        ["a", line([0, 0], [100, 0])],
        ["b", line([20, 0], [80, 0])],
      ]),
      ["a", "b"],
    );

    expect(hops.size).toBe(0);
  });

  it("ignora los cruces pegados a un extremo", () => {
    // Ahi las dos lineas llegan al mismo nodo: no es un cruce, es una union.
    const hops = findHops(
      new Map([
        ["a", line([0, 0], [100, 0])],
        ["b", line([2, -50], [2, 50])],
      ]),
      ["a", "b"],
    );

    expect(hops.size).toBe(0);
  });

  it("la direccion del salto sigue a la linea que pasa por encima", () => {
    const horizontal = findHops(cross(), ["b", "a"]).get("a")![0]!;
    const vertical = findHops(cross(), ["a", "b"]).get("b")![0]!;

    expect(Math.abs(Math.cos(horizontal.angle))).toBeCloseTo(1, 5);
    expect(Math.abs(Math.cos(vertical.angle))).toBeCloseTo(0, 5);
  });

  it("una linea que cruza a dos produce dos saltos", () => {
    const hops = findHops(
      new Map([
        ["a", line([0, 30], [200, 30])],
        ["b", line([0, 90], [200, 90])],
        ["c", line([100, 0], [100, 150])],
      ]),
      ["a", "b", "c"],
    );

    expect(hops.get("c")).toHaveLength(2);
  });

  it("dos cruces casi en el mismo punto se funden en uno", () => {
    // Dos arcos solapados producen una mancha, no un puente.
    const hops = findHops(
      new Map([
        ["a", line([0, 50], [200, 50])],
        ["b", line([0, 52], [200, 52])],
        ["c", line([100, 0], [100, 150])],
      ]),
      ["a", "b", "c"],
    );

    expect(hops.get("c")).toHaveLength(1);
  });

  it("una poligonal que pasa de largo no produce salto", () => {
    // `b` queda por debajo del tramo horizontal de `a`, sin tocarlo.
    const hops = findHops(
      new Map([
        ["a", line([0, 0], [100, 0], [100, 100], [200, 100])],
        ["b", line([150, 110], [150, 200])],
      ]),
      ["a", "b"],
    );

    expect(hops.size).toBe(0);
  });

  it("recorre tambien los tramos intermedios de una poligonal", () => {
    const cruzando = findHops(
      new Map([
        ["a", line([0, 0], [100, 0], [100, 100], [200, 100])],
        ["b", line([50, 50], [150, 50])],
      ]),
      ["a", "b"],
    );

    // El tramo vertical de `a` (x=100, de y=0 a 100) corta a `b` en (100,50).
    expect(cruzando.get("b")![0]).toMatchObject({ x: 100, y: 50 });
  });

  it("una poligonal de un solo punto se ignora", () => {
    const hops = findHops(
      new Map([
        ["a", line([50, 50])],
        ["b", line([0, 50], [100, 50])],
      ]),
      ["a", "b"],
    );

    expect(hops.size).toBe(0);
  });
});

describe("hopPath", () => {
  it("el arco empieza y acaba sobre la linea, a un radio del cruce", () => {
    const d = hopPath({ x: 50, y: 50, angle: 0 });

    expect(d).toBe(
      `M ${50 - HOP_RADIUS} 50 A ${HOP_RADIUS} ${HOP_RADIUS} 0 0 1 ${50 + HOP_RADIUS} 50`,
    );
  });

  it("se orienta con la linea", () => {
    const vertical = hopPath({ x: 50, y: 50, angle: Math.PI / 2 });

    // En vertical el arco se desplaza en Y, no en X.
    expect(vertical).toContain(`M 50 ${50 - HOP_RADIUS}`);
    expect(vertical).toContain(`50 ${50 + HOP_RADIUS}`);
  });

  it("todos los arcos curvan hacia el mismo lado", () => {
    // Con `sweep-flag` variable, los puentes de un mismo diagrama mirarian
    // unos arriba y otros abajo.
    for (const angle of [0, 0.7, Math.PI / 2, Math.PI, -1.2]) {
      expect(hopPath({ x: 10, y: 10, angle })).toContain("0 0 1");
    }
  });
});
