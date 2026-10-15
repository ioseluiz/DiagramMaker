import type { Side } from "@/model/generated/Side";

/**
 * Identificadores de los conectores de React Flow.
 *
 * Un nodo normal tiene cuatro, uno por lado, y su id es el nombre del lado.
 * Una tabla entidad-relacion tiene ademas dos por columna, porque una clave
 * ajena no relaciona dos tablas en abstracto: relaciona **una columna con
 * otra**. El id de esos lleva delante el de la columna.
 *
 *   "right"              -> el lado derecho del nodo
 *   "c_a1b2@right"       -> el lado derecho de la fila de esa columna
 *
 * La arroba no puede aparecer en un id de columna, que se genera con
 * `newId()` a partir de letras, digitos y guiones bajos.
 */

const SEPARATOR = "@";

export interface HandleRef {
  side: Side;
  /** Columna concreta, si el conector es el de una fila. */
  column?: string;
}

export function encodeHandle(side: Side, column?: string): string {
  return column ? `${column}${SEPARATOR}${side}` : side;
}

const SIDES = new Set<string>(["top", "right", "bottom", "left"]);

export function decodeHandle(id: string | null | undefined): HandleRef | null {
  if (!id) return null;

  const at = id.lastIndexOf(SEPARATOR);
  if (at < 0) {
    return SIDES.has(id) ? { side: id as Side } : null;
  }

  const side = id.slice(at + 1);
  const column = id.slice(0, at);
  if (!SIDES.has(side) || !column) return null;

  return { side: side as Side, column };
}
