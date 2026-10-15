import type { Rect } from "@/model/generated/Rect";
import type { Side } from "@/model/generated/Side";

/**
 * Lado por el que una arista toca cada caja.
 *
 * El lado que elige el usuario al arrastrar se guarda en el documento
 * (`Endpoint.side`) y manda siempre. Esto solo resuelve el caso en que no hay
 * ninguno elegido: diagramas importados desde DBML, Mermaid o PlantUML, donde
 * nadie ha tocado nada.
 *
 * Los ids de handle de React Flow son exactamente los nombres de `Side`, asi
 * que no hace falta traducir entre ambos.
 */

export const SIDES: readonly Side[] = ["top", "right", "bottom", "left"];

export interface SidePair {
  source: Side;
  target: Side;
}

/** Hueco entre dos intervalos; 0 o negativo si se solapan. */
function gap(aMin: number, aMax: number, bMin: number, bMax: number): number {
  return Math.max(bMin - aMax, aMin - bMax);
}

/**
 * Elige el par de lados a partir de la geometria.
 *
 * Conecta por el eje en el que las cajas estan realmente separadas: si una
 * esta claramente a la derecha de la otra, sale por los costados; si esta
 * encima o debajo, por arriba o abajo. Mirar la separacion y no la distancia
 * entre centros es lo que acierta con cajas anchas: dos rectangulos de 300 px
 * apilados tienen los centros casi alineados, pero el hueco real es vertical.
 */
export function chooseSides(source: Rect, target: Rect): SidePair {
  const horizontal = gap(
    source.x,
    source.x + source.width,
    target.x,
    target.x + target.width,
  );
  const vertical = gap(
    source.y,
    source.y + source.height,
    target.y,
    target.y + target.height,
  );

  if (horizontal >= vertical) {
    const targetIsRight = target.x + target.width / 2 >= source.x + source.width / 2;
    return targetIsRight
      ? { source: "right", target: "left" }
      : { source: "left", target: "right" };
  }

  const targetIsBelow = target.y + target.height / 2 >= source.y + source.height / 2;
  return targetIsBelow
    ? { source: "bottom", target: "top" }
    : { source: "top", target: "bottom" };
}

/** Par de lados de un bucle sobre el propio nodo. */
export const SELF_LOOP_SIDES: SidePair = { source: "right", target: "top" };

/**
 * Resuelve los lados de una arista: lo elegido gana, la geometria rellena.
 */
export function resolveSides(
  stored: { source?: Side; target?: Side },
  source: Rect | undefined,
  target: Rect | undefined,
  selfLoop: boolean,
): SidePair {
  const fallback = selfLoop
    ? SELF_LOOP_SIDES
    : source && target
      ? chooseSides(source, target)
      : // Sin geometria no hay nada que calcular; abajo-arriba es la lectura
        // natural de un diagrama de flujo.
        { source: "bottom" as Side, target: "top" as Side };

  return {
    source: stored.source ?? fallback.source,
    target: stored.target ?? fallback.target,
  };
}
