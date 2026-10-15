/**
 * Medidas de una tabla ER.
 *
 * Espejo de las constantes `TABLE_*` de
 * `src-tauri/src/model/shape_geometry.rs`. Viven en su propio modulo para que
 * tanto la geometria (`shapeGeometry.ts`) como el calculo de tamano
 * (`table.ts`) partan del mismo sitio, y no de dos copias.
 *
 * `shapeGeometry.test.ts` compara las primitivas resultantes contra el fichero
 * dorado que genera Rust, asi que un cambio en una sola de las dos partes
 * rompe una prueba.
 */

export const TABLE_HEADER_HEIGHT = 28;
export const TABLE_ROW_HEIGHT = 22;
export const TABLE_PADDING = 10;
export const TABLE_HEADER_FONT_SIZE = 13;
export const TABLE_ROW_FONT_SIZE = 12;

export function tableHeight(rows: number): number {
  return TABLE_HEADER_HEIGHT + rows * TABLE_ROW_HEIGHT;
}
