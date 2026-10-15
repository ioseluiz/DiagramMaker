import { describe, expect, it } from "vitest";

import type { Rect } from "@/model/generated/Rect";
import { SELF_LOOP_SIDES, chooseSides, resolveSides } from "./sides";

const box = (x: number, y: number, width = 160, height = 60): Rect => ({
  x,
  y,
  width,
  height,
});

describe("chooseSides", () => {
  it("cajas lado a lado se conectan por los costados", () => {
    expect(chooseSides(box(0, 0), box(300, 0))).toEqual({
      source: "right",
      target: "left",
    });
  });

  it("invierte los costados si el destino esta a la izquierda", () => {
    expect(chooseSides(box(300, 0), box(0, 0))).toEqual({
      source: "left",
      target: "right",
    });
  });

  it("cajas apiladas se conectan por arriba y abajo", () => {
    expect(chooseSides(box(0, 0), box(0, 200))).toEqual({
      source: "bottom",
      target: "top",
    });
  });

  it("invierte el eje vertical si el destino esta encima", () => {
    expect(chooseSides(box(0, 200), box(0, 0))).toEqual({
      source: "top",
      target: "bottom",
    });
  });

  it("dos cajas anchas apiladas no se confunden con horizontales", () => {
    // Es el caso que falla si se compara la distancia entre centros: los
    // centros estan a 40 px en X y a 200 en Y, pero en X las cajas se solapan.
    expect(chooseSides(box(0, 0, 300, 60), box(40, 200, 300, 60))).toEqual({
      source: "bottom",
      target: "top",
    });
  });

  it("dos cajas altas en paralelo salen por los costados", () => {
    expect(chooseSides(box(0, 0, 60, 300), box(200, 40, 60, 300))).toEqual({
      source: "right",
      target: "left",
    });
  });

  it("en diagonal gana el eje con mas separacion", () => {
    // Hueco horizontal 240, vertical 40: manda el horizontal.
    expect(chooseSides(box(0, 0), box(400, 100))).toEqual({
      source: "right",
      target: "left",
    });

    // Hueco horizontal 40, vertical 240: manda el vertical.
    expect(chooseSides(box(0, 0), box(200, 300))).toEqual({
      source: "bottom",
      target: "top",
    });
  });

  it("cajas que se solapan siguen dando un par utilizable", () => {
    const pair = chooseSides(box(0, 0), box(20, 20));

    expect(pair.source).not.toBe(pair.target);
  });

  it("nunca devuelve el mismo lado en ambos extremos", () => {
    const posiciones = [-400, -200, -40, 0, 40, 200, 400];

    for (const x of posiciones) {
      for (const y of posiciones) {
        const pair = chooseSides(box(0, 0), box(x, y));
        expect(pair.source, `(${x},${y})`).not.toBe(pair.target);
      }
    }
  });
});

describe("resolveSides", () => {
  const a = box(0, 0);
  const b = box(300, 0);

  it("el lado elegido por el usuario manda sobre la geometria", () => {
    const pair = resolveSides({ source: "top", target: "bottom" }, a, b, false);

    expect(pair).toEqual({ source: "top", target: "bottom" });
  });

  it("rellena solo el extremo que falta", () => {
    const pair = resolveSides({ source: "top" }, a, b, false);

    expect(pair.source).toBe("top");
    expect(pair.target).toBe("left");
  });

  it("sin nada elegido usa la geometria", () => {
    expect(resolveSides({}, a, b, false)).toEqual({
      source: "right",
      target: "left",
    });
  });

  it("un bucle sobre si mismo usa su par propio", () => {
    expect(resolveSides({}, a, a, true)).toEqual(SELF_LOOP_SIDES);
  });

  it("sin geometria cae en abajo-arriba", () => {
    expect(resolveSides({}, undefined, undefined, false)).toEqual({
      source: "bottom",
      target: "top",
    });
  });
});
