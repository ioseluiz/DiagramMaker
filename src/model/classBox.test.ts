import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

import type { Primitive } from "./shapeGeometry";
import type { Rect } from "./generated/Rect";
import type { Visibility } from "./generated/Visibility";
import { SIGIL, memberText, visibilityOf } from "./classBox";
import {
  classAttributesTop,
  classCompartmentHeight,
  classHeight,
  classOperationsTop,
  classRowRect,
} from "./classMetrics";
import { classPrimitivesFor } from "./shapeGeometry";

/**
 * Gemela de `src-tauri/tests/golden_class.rs`: las dos leen los mismos JSON.
 * Si la geometria o el texto de un miembro cambian en una sola de las dos
 * implementaciones, una de las dos pruebas falla.
 */

function readGolden<T>(name: string): T {
  return JSON.parse(
    readFileSync(resolve(process.cwd(), `src-tauri/tests/golden/${name}`), "utf8"),
  ) as T;
}

interface BoxCase {
  stereotype: boolean;
  attributes: number;
  operations: number;
  width: number;
  height: number;
  attributesTop: number;
  operationsTop: number;
  firstAttribute: Rect;
  firstOperation: Rect;
  primitives: Primitive[];
}

interface MemberCase {
  description: string;
  isOperation: boolean;
  text: string;
}

const boxes = readGolden<BoxCase[]>("class-primitives.json");
const members = readGolden<MemberCase[]>("class-members.json");

/**
 * Reconstruye el miembro de cada caso dorado a partir de su texto esperado.
 *
 * No se puede: el fichero solo guarda el resultado. Lo que se hace es montar
 * aqui los mismos miembros que monta Rust y comprobar que dan ese texto; la
 * descripcion es la clave que los empareja.
 */
const FIXTURES: Record<string, { member: Parameters<typeof memberText>[0] }> = {
  "atributo con tipo": {
    member: {
      id: "a",
      name: "nombre",
      visibility: "private",
      typeName: "String",
      parameters: "",
    },
  },
  "atributo sin tipo": {
    member: {
      id: "a",
      name: "activo",
      visibility: "public",
      typeName: "",
      parameters: "",
    },
  },
  "atributo protegido": {
    member: {
      id: "a",
      name: "saldo",
      visibility: "protected",
      typeName: "Decimal",
      parameters: "",
    },
  },
  "atributo de paquete": {
    member: {
      id: "a",
      name: "interno",
      visibility: "package",
      typeName: "",
      parameters: "",
    },
  },
  "operacion sin nada": {
    member: {
      id: "o",
      name: "guardar",
      visibility: "public",
      typeName: "",
      parameters: "",
    },
  },
  "operacion con parametros y retorno": {
    member: {
      id: "o",
      name: "buscar",
      visibility: "public",
      typeName: "Pedido",
      parameters: "id: int, activo: bool",
    },
  },
  "operacion que no devuelve nada": {
    member: {
      id: "o",
      name: "borrar",
      visibility: "private",
      typeName: "",
      parameters: "id: int",
    },
  },
};

describe("geometria de la caja de clase contra el fichero dorado de Rust", () => {
  it("el fichero trae casos", () => {
    expect(boxes.length).toBeGreaterThan(4);
  });

  it.each(
    boxes.map(
      (c) =>
        [
          `estereotipo=${c.stereotype} atributos=${c.attributes} operaciones=${c.operations}`,
          c,
        ] as const,
    ),
  )("%s", (_name, testCase) => {
    expect(classAttributesTop(testCase.stereotype)).toBe(testCase.attributesTop);
    expect(classOperationsTop(testCase.stereotype, testCase.attributes)).toBe(
      testCase.operationsTop,
    );
    expect(
      classHeight(testCase.stereotype, testCase.attributes, testCase.operations),
    ).toBe(testCase.height);

    expect(classRowRect(testCase.attributesTop, 0, testCase.width)).toEqual(
      testCase.firstAttribute,
    );
    expect(classRowRect(testCase.operationsTop, 0, testCase.width)).toEqual(
      testCase.firstOperation,
    );

    expect(
      classPrimitivesFor(
        testCase.stereotype,
        testCase.attributes,
        testCase.width,
        testCase.height,
      ),
    ).toEqual(testCase.primitives);
  });
});

describe("texto de un miembro contra el fichero dorado de Rust", () => {
  it("el fichero trae casos", () => {
    expect(members.length).toBeGreaterThan(5);
  });

  it.each(members.map((c) => [c.description, c] as const))(
    "%s",
    (description, testCase) => {
      const fixture = FIXTURES[description];
      expect(fixture, `falta el caso \`${description}\` en FIXTURES`).toBeDefined();

      expect(memberText(fixture!.member, testCase.isOperation)).toBe(testCase.text);
    },
  );
});

describe("visibilidad", () => {
  it("cada signo vuelve a su visibilidad", () => {
    for (const visibility of Object.keys(SIGIL) as Visibility[]) {
      expect(visibilityOf(SIGIL[visibility])).toBe(visibility);
    }
  });

  it("un signo que no es ninguno no devuelve nada", () => {
    expect(visibilityOf("x")).toBeUndefined();
  });
});

describe("compartimentos", () => {
  it("uno vacio sigue ocupando sitio", () => {
    // UML dibuja los tres siempre: una clase sin operaciones tiene que
    // seguir viendose como una clase.
    expect(classCompartmentHeight(0)).toBeGreaterThan(0);
  });

  it("cada fila anade su alto", () => {
    expect(classCompartmentHeight(3) - classCompartmentHeight(2)).toBe(
      classCompartmentHeight(2) - classCompartmentHeight(1),
    );
  });
});
