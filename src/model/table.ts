import type { ErColumn } from "./generated/ErColumn";
import type { ErTable } from "./generated/ErTable";
import type { Node } from "./generated/Node";
import { measureWidth } from "@/canvas/text";
import {
  TABLE_HEADER_FONT_SIZE,
  TABLE_HEADER_HEIGHT,
  TABLE_PADDING,
  TABLE_ROW_FONT_SIZE,
  TABLE_ROW_HEIGHT,
  tableHeight,
} from "./tableMetrics";

/**
 * Tamano y contenido de una tabla ER.
 *
 * Lo unico que exige medir texto —el ancho— vive aqui y no en Rust: el
 * navegador es quien conoce la tipografia real. Las medidas fijas vienen de
 * `tableMetrics`, que es el espejo de las constantes de Rust.
 */

export {
  TABLE_HEADER_FONT_SIZE,
  TABLE_HEADER_HEIGHT,
  TABLE_PADDING,
  TABLE_ROW_FONT_SIZE,
  TABLE_ROW_HEIGHT,
  tableHeight,
};

/** Separacion minima entre el nombre de la columna y su tipo. */
const COLUMN_GAP = 24;
const MIN_WIDTH = 160;
const MAX_WIDTH = 420;

/** Marcas que se muestran junto al tipo. Espejo de `ErColumn::badges`. */
export function columnBadges(column: ErColumn): string[] {
  const out: string[] = [];
  if (column.primaryKey) out.push("PK");
  if (column.foreignKey) out.push("FK");
  if (column.unique && !column.primaryKey) out.push("U");
  if (!column.nullable && !column.primaryKey) out.push("NN");
  return out;
}

/** Texto que va a la derecha de la fila: tipo mas marcas. */
export function columnDetail(column: ErColumn): string {
  const badges = columnBadges(column);
  return [column.dataType, ...badges].filter(Boolean).join(" ");
}

/**
 * Ancho que necesita la tabla para que no se solapen nombre y tipo.
 *
 * Es lo unico de la tabla que exige medir texto, y por eso vive aqui y no en
 * Rust: el navegador es quien conoce la tipografia real.
 */
export function tableWidth(label: string, table: ErTable): number {
  let widest = measureWidth(label, TABLE_HEADER_FONT_SIZE, true);

  for (const column of table.columns) {
    const name = measureWidth(column.name, TABLE_ROW_FONT_SIZE, column.primaryKey);
    const detail = measureWidth(columnDetail(column), TABLE_ROW_FONT_SIZE);
    widest = Math.max(widest, name + (detail > 0 ? COLUMN_GAP + detail : 0));
  }

  const total = widest + TABLE_PADDING * 2;
  return Math.round(Math.min(Math.max(total, MIN_WIDTH), MAX_WIDTH));
}

/** Tamano completo que necesita un nodo de tabla. */
export function tableSize(node: Node): { width: number; height: number } {
  const table = node.data.type === "er" ? node.data : null;
  if (!table) return { width: MIN_WIDTH, height: tableHeight(0) };

  return {
    width: tableWidth(node.label, table),
    height: tableHeight(table.columns.length),
  };
}

/** Caja de la fila `index`, relativa al nodo. */
export function rowRect(index: number, width: number) {
  return {
    x: 0,
    y: TABLE_HEADER_HEIGHT + index * TABLE_ROW_HEIGHT,
    width,
    height: TABLE_ROW_HEIGHT,
  };
}

/** `true` si el nodo es una tabla ER. */
export function erTableOf(node: Node): ErTable | null {
  return node.data.type === "er" ? node.data : null;
}
