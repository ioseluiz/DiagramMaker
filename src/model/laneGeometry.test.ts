import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import type { LaneLayout } from "./generated/LaneLayout";
import type { Measures } from "./generated/Measures";
import type { SequenceScript } from "./generated/SequenceScript";
import { MIN_HEADER_WIDTH, layout, messagesOf, walkSteps } from "./laneGeometry";

/**
 * Gemela de `src-tauri/tests/golden_lane.rs`.
 *
 * Las dos leen el mismo JSON. Si el reparto de Rust y el de TypeScript dejan
 * de coincidir, una de las dos falla; si cambian los dos a la vez de la misma
 * manera, hay que regenerar el fichero con
 * `UPDATE_GOLDEN=1 cargo test --manifest-path src-tauri/Cargo.toml golden`.
 */

interface Case {
  name: string;
  participants: string[];
  script: SequenceScript;
  measures: Measures;
  layout: LaneLayout;
}

const GOLDEN = resolve(process.cwd(), "src-tauri/tests/golden/lane-layout.json");

const cases: Case[] = JSON.parse(readFileSync(GOLDEN, "utf8")) as Case[];

describe("reparto de secuencia contra el fichero dorado de Rust", () => {
  it("el fichero trae casos", () => {
    // Si el JSON se vacia o se mueve, el `it.each` de abajo no correria
    // ninguna prueba y esto pasaria desapercibido.
    expect(cases.length).toBeGreaterThan(5);
  });

  it.each(cases.map((c) => [c.name, c] as const))("%s", (_name, testCase) => {
    const got = layout(testCase.participants, testCase.script, testCase.measures);

    expect(got).toEqual(testCase.layout);
  });
});

describe("recorrido del guion", () => {
  const anidado = cases.find((c) => c.name === "marcos anidados")!;

  it("entra en los marcos en orden de lectura", () => {
    const ids = walkSteps(anidado.script.steps).map((s) => s.id);

    expect(ids).toEqual(["fuera", "m1", "dentro", "m2", "m3"]);
  });

  it("saca los mensajes de dentro de los marcos", () => {
    expect(messagesOf(anidado.script).map((m) => m.id)).toEqual(["m1", "m2", "m3"]);
  });
});

describe("casos que el fichero dorado no cubre", () => {
  const vacio: Measures = { headers: {}, texts: {} };
  const sinPasos: SequenceScript = { steps: [] };

  it("sin participantes devuelve un reparto vacio", () => {
    const out = layout([], sinPasos, vacio);

    expect(out.lanes).toEqual([]);
    expect(out.size.width).toBe(48);
  });

  it("un participante sin medir usa el ancho minimo", () => {
    const out = layout(["a"], sinPasos, vacio);

    expect(out.lanes[0]!.header.width).toBe(MIN_HEADER_WIDTH);
  });

  it("un mensaje a un participante que no existe se ignora", () => {
    const script: SequenceScript = {
      steps: [
        { type: "message", id: "m", from: "a", to: "fantasma", kind: "sync", text: "" },
      ],
    };

    expect(layout(["a"], script, vacio).arrows).toEqual([]);
  });
});
