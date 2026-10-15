import type { Edge } from "./generated/Edge";
import type { EdgeStyle } from "./generated/EdgeStyle";
import type { Node } from "./generated/Node";
import type { NodeStyle } from "./generated/NodeStyle";

/**
 * Estilos resueltos, listos para pasar a React.
 *
 * El formato en disco omite los estilos por defecto y escribe `null` en los
 * campos sin valor, que es lo correcto para que un `.dgm` produzca diffs
 * legibles. React, en cambio, quiere `undefined` para "no fijes esta
 * propiedad" — `null` lo trataria como un valor. Esta conversion ocurre aqui,
 * una sola vez, en lugar de con un `?? undefined` repetido en cada componente.
 */

export type Resolved<T> = { [K in keyof T]-?: Exclude<T[K], null> | undefined };

function undefineNulls<T extends object>(source: T | null | undefined): Resolved<T> {
  const out = {} as Record<string, unknown>;
  for (const [key, value] of Object.entries(source ?? {})) {
    out[key] = value ?? undefined;
  }
  return out as Resolved<T>;
}

export function nodeStyle(node: Node): Resolved<NodeStyle> {
  return undefineNulls(node.style);
}

export function edgeStyle(edge: Edge): Resolved<EdgeStyle> {
  return undefineNulls(edge.style);
}
