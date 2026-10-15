import type { Point } from "@/model/generated/Point";

/**
 * Saltos de linea en los cruces entre relaciones.
 *
 * Cuando dos lineas se cruzan sin estar conectadas, la convencion de los
 * programas de diagramas —draw.io la llama *line jump*— es dibujar un
 * puentecito en una de las dos para que se vea cual pasa por encima. Sin eso,
 * un cruce y una union se dibujan igual y el diagrama miente.
 *
 * El cruce se calcula sobre una poligonal aproximada de cada trazado, no
 * sobre su curva exacta. Un salto es un arco de cinco pixeles: errar por uno
 * o dos no se ve, y a cambio el calculo es geometria elemental y se puede
 * probar.
 */

export interface Hop {
  x: number;
  y: number;
  /** Direccion de la linea en el cruce, en radianes. El arco se orienta con ella. */
  angle: number;
}

/** Radio del puente. Mas pequeno se lee como un punto, no como un puente. */
export const HOP_RADIUS = 8;

/**
 * Dos cruces mas juntos que esto se funden en uno: dibujar dos arcos
 * solapados produce una mancha, no un puente.
 */
const MIN_SEPARATION = HOP_RADIUS * 2.5;

/**
 * Margen por el que se ignora un cruce pegado a un extremo. Ahi las lineas
 * convergen en el mismo nodo y el puente sobraria.
 */
const ENDPOINT_MARGIN = 12;

interface Segment {
  a: Point;
  b: Point;
}

function segments(points: readonly Point[]): Segment[] {
  const out: Segment[] = [];
  for (let i = 1; i < points.length; i++) {
    out.push({ a: points[i - 1]!, b: points[i]! });
  }
  return out;
}

/** Punto donde se cortan dos segmentos, o `null` si no se cortan. */
function intersection(p: Segment, q: Segment): Point | null {
  const r = { x: p.b.x - p.a.x, y: p.b.y - p.a.y };
  const s = { x: q.b.x - q.a.x, y: q.b.y - q.a.y };

  const denominator = r.x * s.y - r.y * s.x;
  // Paralelos o degenerados: aunque se solapen, un puente no aclararia nada.
  if (Math.abs(denominator) < 1e-9) return null;

  const delta = { x: q.a.x - p.a.x, y: q.a.y - p.a.y };
  const t = (delta.x * s.y - delta.y * s.x) / denominator;
  const u = (delta.x * r.y - delta.y * r.x) / denominator;

  if (t < 0 || t > 1 || u < 0 || u > 1) return null;

  return { x: p.a.x + t * r.x, y: p.a.y + t * r.y };
}

function distance(a: Point, b: Point): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

/** `true` si el punto cae demasiado cerca de donde empieza o acaba la linea. */
function nearEnds(point: Point, polyline: readonly Point[]): boolean {
  const first = polyline[0];
  const last = polyline[polyline.length - 1];

  return (
    (first !== undefined && distance(point, first) < ENDPOINT_MARGIN) ||
    (last !== undefined && distance(point, last) < ENDPOINT_MARGIN)
  );
}

/**
 * Calcula los saltos de cada linea.
 *
 * `order` fija quien pasa por encima: de cada par que se cruza, salta la que
 * aparece despues, que es la que se dibuja encima. Asi el resultado no
 * depende del orden en que se recorran los pares y no parpadea entre
 * repintados.
 */
export function findHops(
  polylines: ReadonlyMap<string, readonly Point[]>,
  order: readonly string[],
): Map<string, Hop[]> {
  const hops = new Map<string, Hop[]>();

  for (let i = 0; i < order.length; i++) {
    const upperId = order[i]!;
    const upper = polylines.get(upperId);
    if (!upper || upper.length < 2) continue;

    const upperSegments = segments(upper);
    const found: Hop[] = [];

    for (let j = 0; j < i; j++) {
      const lower = polylines.get(order[j]!);
      if (!lower || lower.length < 2) continue;

      for (const p of upperSegments) {
        for (const q of segments(lower)) {
          const point = intersection(p, q);
          if (!point) continue;
          if (nearEnds(point, upper) || nearEnds(point, lower)) continue;

          found.push({
            x: point.x,
            y: point.y,
            angle: Math.atan2(p.b.y - p.a.y, p.b.x - p.a.x),
          });
        }
      }
    }

    const merged = mergeClose(found);
    if (merged.length > 0) hops.set(upperId, merged);
  }

  return hops;
}

/** Funde los cruces demasiado proximos entre si. */
function mergeClose(hops: readonly Hop[]): Hop[] {
  const out: Hop[] = [];

  for (const hop of hops) {
    const tooClose = out.some((kept) => distance(kept, hop) < MIN_SEPARATION);
    if (!tooClose) out.push(hop);
  }

  return out;
}

/**
 * Trazado del puente: un arco que sale de la linea, la rodea y vuelve.
 *
 * Se devuelve suelto, para dibujarlo encima del trazado en lugar de
 * reescribirlo. Rehacer el `d` de la arista obligaria a partir curvas, y un
 * arco superpuesto se ve igual con geometria elemental.
 */
export function hopPath(hop: Hop, radius = HOP_RADIUS): string {
  const dx = Math.cos(hop.angle) * radius;
  const dy = Math.sin(hop.angle) * radius;

  const from = { x: hop.x - dx, y: hop.y - dy };
  const to = { x: hop.x + dx, y: hop.y + dy };

  // `sweep-flag` 1 curva el arco hacia el mismo lado siempre, de modo que
  // todos los puentes de un diagrama miran igual.
  return `M ${round(from.x)} ${round(from.y)} A ${radius} ${radius} 0 0 1 ${round(to.x)} ${round(to.y)}`;
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}
