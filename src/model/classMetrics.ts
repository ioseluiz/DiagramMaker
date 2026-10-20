import type { Rect } from "./generated/Rect";

/**
 * Medidas de una caja de clase.
 *
 * Espejo de las constantes `CLASS_*` de
 * `src-tauri/src/model/shape_geometry.rs`. Viven en su propio modulo por lo
 * mismo que las de la tabla: tanto la geometria (`shapeGeometry.ts`) como el
 * calculo de tamano (`classBox.ts`) tienen que partir del mismo sitio.
 *
 * `shapeGeometry.test.ts` compara el resultado contra el fichero dorado que
 * genera Rust, asi que cambiar una sola de las dos partes rompe una prueba.
 */

export const CLASS_HEADER_HEIGHT = 30;
export const CLASS_STEREOTYPE_HEIGHT = 15;
export const CLASS_ROW_HEIGHT = 20;
export const CLASS_EMPTY_HEIGHT = 10;
export const CLASS_PADDING = 10;
export const CLASS_NAME_FONT_SIZE = 13;
export const CLASS_STEREOTYPE_FONT_SIZE = 11;
export const CLASS_MEMBER_FONT_SIZE = 12;

export function classHeaderHeight(stereotype: boolean): number {
  return stereotype
    ? CLASS_HEADER_HEIGHT + CLASS_STEREOTYPE_HEIGHT
    : CLASS_HEADER_HEIGHT;
}

/**
 * Alto de un compartimento.
 *
 * Sin filas no es cero: UML dibuja los tres compartimentos siempre, y una
 * clase sin operaciones tiene que seguir viendose como una clase.
 */
export function classCompartmentHeight(rows: number): number {
  return rows === 0 ? CLASS_EMPTY_HEIGHT : rows * CLASS_ROW_HEIGHT;
}

export function classAttributesTop(stereotype: boolean): number {
  return classHeaderHeight(stereotype);
}

export function classOperationsTop(stereotype: boolean, attributes: number): number {
  return classAttributesTop(stereotype) + classCompartmentHeight(attributes);
}

export function classHeight(
  stereotype: boolean,
  attributes: number,
  operations: number,
): number {
  return (
    classOperationsTop(stereotype, attributes) + classCompartmentHeight(operations)
  );
}

/** Caja de la fila `index` de un compartimento que empieza en `top`. */
export function classRowRect(top: number, index: number, w: number): Rect {
  return {
    x: CLASS_PADDING,
    y: top + index * CLASS_ROW_HEIGHT,
    width: Math.max(w - CLASS_PADDING * 2, 0),
    height: CLASS_ROW_HEIGHT,
  };
}
