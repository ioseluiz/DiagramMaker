import type { NodeShape } from "./generated/NodeShape";
import type { Primitive } from "./generated/Primitive";
import { TABLE_HEADER_HEIGHT, TABLE_ROW_HEIGHT } from "./tableMetrics";
import { classAttributesTop, classOperationsTop } from "./classMetrics";

/**
 * Contorno de cada forma, en primitivas SVG.
 *
 * Reimplementa `src-tauri/src/model/shape_geometry.rs` porque el lienzo
 * repinta decenas de nodos por fotograma y no puede cruzar el puente IPC para
 * cada uno. La duplicacion esta vigilada: `shapeGeometry.test.ts` y
 * `src-tauri/tests/golden_shapes.rs` comparan ambas implementaciones contra el
 * mismo `src-tauri/tests/golden/shape-primitives.json`, asi que si una cambia
 * sin la otra, falla una prueba.
 *
 * **Cualquier cambio aqui tiene que hacerse tambien en el lado de Rust.**
 */

export type { Primitive };

const CORNER_RADIUS = 10;
/** Radio de las esquinas de una tabla ER. */
const TABLE_CORNER_RADIUS = 6;
/** Alto de la solapa del paquete. */
const TAB_HEIGHT = 22;
/** Alto maximo de la tapa eliptica del cilindro. */
const CYLINDER_LIP = 12;
/** Hueco entre el anillo de un nodo final y el disco de dentro. */
const FINAL_NODE_GAP = 4;
/** Grosor de la barra de bifurcacion. */
const BAR_THICKNESS = 6;

/**
 * El trazo se dibuja centrado en el borde, asi que medio grosor se sale de la
 * caja. Encogiendo este margen por lado, el contorno queda dentro.
 */
const INSET = 0.75;

/**
 * Redondea a 4 decimales. Sin esto, el mismo calculo en Rust y en JavaScript
 * diverge en el ultimo bit y el fichero dorado deja de coincidir por ruido.
 */
function r(value: number): number {
  return Math.round(value * 10_000) / 10_000;
}

export const ALL_SHAPES: readonly NodeShape[] = [
  "rectangle",
  "roundedRect",
  "ellipse",
  "diamond",
  "cylinder",
  "note",
  "package",
  "actor",
  "hexagon",
  "parallelogram",
  "initialNode",
  "finalNode",
  "bar",
];

/** `true` si la primitiva se rellena. Las lineas interiores no. */
export function isFilled(primitive: Primitive): boolean {
  return primitive.kind !== "line";
}

/**
 * `true` si se rellena con el color de la linea en vez del de la superficie:
 * el disco de un nodo inicial, la barra de una bifurcacion.
 */
export function isSolid(primitive: Primitive): boolean {
  return primitive.kind === "solid";
}

/**
 * Un circulo escrito como trazo cerrado, para poder rellenarlo con el color
 * de la linea. Espejo de `circle_path` en `shape_geometry.rs`.
 */
function circlePath(cx: number, cy: number, radius: number): string {
  const rad = r(radius);
  return (
    `M ${r(cx - radius)} ${r(cy)} A ${rad} ${rad} 0 1 0 ${r(cx + radius)} ${r(cy)} ` +
    `A ${rad} ${rad} 0 1 0 ${r(cx - radius)} ${r(cy)} Z`
  );
}

/**
 * Contorno de `shape` dentro de una caja de `w` x `h`, en coordenadas locales
 * (origen en la esquina superior izquierda de la caja).
 */
export function primitivesFor(shape: NodeShape, w: number, h: number): Primitive[] {
  const i = INSET;

  switch (shape) {
    case "rectangle":
      return [
        { kind: "rect", x: r(i), y: r(i), width: r(w - i * 2), height: r(h - i * 2) },
      ];

    case "roundedRect":
      return [
        {
          kind: "rect",
          x: r(i),
          y: r(i),
          width: r(w - i * 2),
          height: r(h - i * 2),
          radius: CORNER_RADIUS,
        },
      ];

    // El sesgo se limita a un cuarto del ancho: en una caja estrecha, un
    // sesgo proporcional al alto se la comeria entera.
    case "hexagon": {
      const cut = Math.min(h / 2, w / 4);
      return [
        {
          kind: "path",
          d:
            `M ${r(i + cut)} ${r(i)} L ${r(w - i - cut)} ${r(i)} ` +
            `L ${r(w - i)} ${r(h / 2)} L ${r(w - i - cut)} ${r(h - i)} ` +
            `L ${r(i + cut)} ${r(h - i)} L ${r(i)} ${r(h / 2)} Z`,
        },
      ];
    }

    case "parallelogram": {
      const skew = Math.min(h / 2, w / 4);
      return [
        {
          kind: "path",
          d:
            `M ${r(i + skew)} ${r(i)} L ${r(w - i)} ${r(i)} ` +
            `L ${r(w - i - skew)} ${r(h - i)} L ${r(i)} ${r(h - i)} Z`,
        },
      ];
    }

    case "ellipse":
      return [
        {
          kind: "ellipse",
          cx: r(w / 2),
          cy: r(h / 2),
          rx: r(w / 2 - i),
          ry: r(h / 2 - i),
        },
      ];

    case "diamond":
      return [
        {
          kind: "path",
          d:
            `M ${r(w / 2)} ${r(i)} L ${r(w - i)} ${r(h / 2)} ` +
            `L ${r(w / 2)} ${r(h - i)} L ${r(i)} ${r(h / 2)} Z`,
        },
      ];

    case "cylinder": {
      const ry = Math.min(CYLINDER_LIP, h / 4);
      const rx = w / 2 - i;
      return [
        {
          kind: "path",
          d:
            `M ${r(i)} ${r(ry + i)} A ${r(rx)} ${r(ry)} 0 0 1 ${r(w - i)} ${r(ry + i)} ` +
            `L ${r(w - i)} ${r(h - ry - i)} ` +
            `A ${r(rx)} ${r(ry)} 0 0 1 ${r(i)} ${r(h - ry - i)} Z`,
        },
        {
          kind: "line",
          d: `M ${r(i)} ${r(ry + i)} A ${r(rx)} ${r(ry)} 0 0 0 ${r(w - i)} ${r(ry + i)}`,
        },
      ];
    }

    case "note": {
      const fold = Math.min(18, w / 4, h / 3);
      return [
        {
          kind: "path",
          d:
            `M ${r(i)} ${r(i)} L ${r(w - fold - i)} ${r(i)} L ${r(w - i)} ${r(fold + i)} ` +
            `L ${r(w - i)} ${r(h - i)} L ${r(i)} ${r(h - i)} Z`,
        },
        {
          kind: "line",
          d:
            `M ${r(w - fold - i)} ${r(i)} L ${r(w - fold - i)} ${r(fold + i)} ` +
            `L ${r(w - i)} ${r(fold + i)}`,
        },
      ];
    }

    case "package": {
      const tabWidth = Math.min(w * 0.45, 120);
      return [
        {
          kind: "path",
          d:
            `M ${r(i)} ${r(TAB_HEIGHT + i)} L ${r(i)} ${r(i)} ` +
            `L ${r(tabWidth)} ${r(i)} L ${r(tabWidth)} ${r(TAB_HEIGHT + i)}`,
        },
        {
          kind: "rect",
          x: r(i),
          y: r(TAB_HEIGHT + i),
          width: r(w - i * 2),
          height: r(h - TAB_HEIGHT - i * 2),
        },
      ];
    }

    // La tabla depende de sus columnas, no solo de la caja: la dibuja
    // `tablePrimitivesFor`. Aqui se devuelve el marco para que un nodo mal
    // formado siga siendo visible en vez de desaparecer.
    case "table":
      return tablePrimitivesFor(0, w, h);

    // Lo mismo con la clase: su contorno depende de cuantos miembros tiene.
    case "class":
      return classPrimitivesFor(false, 0, w, h);

    case "initialNode": {
      const radius = Math.max(Math.min(w, h) / 2 - i, 1);
      return [{ kind: "solid", d: circlePath(w / 2, h / 2, radius) }];
    }

    case "finalNode": {
      const outer = Math.max(Math.min(w, h) / 2 - i, 1);
      const inner = Math.max(outer - FINAL_NODE_GAP, 1);
      return [
        { kind: "circle", cx: r(w / 2), cy: r(h / 2), r: r(outer) },
        { kind: "solid", d: circlePath(w / 2, h / 2, inner) },
      ];
    }

    case "bar": {
      // Tumbada si la caja es mas ancha que alta, de pie si no.
      const [x, y, width, height] =
        w >= h
          ? [i, (h - BAR_THICKNESS) / 2, w - i * 2, BAR_THICKNESS]
          : [(w - BAR_THICKNESS) / 2, i, BAR_THICKNESS, h - i * 2];

      const right = r(x + Math.max(width, 1));
      const bottom = r(y + Math.max(height, 1));
      return [
        {
          kind: "solid",
          d: `M ${r(x)} ${r(y)} L ${right} ${r(y)} L ${right} ${bottom} L ${r(x)} ${bottom} Z`,
        },
      ];
    }

    // El texto suelto no tiene contorno: es solo su etiqueta.
    case "text":
      return [];

    case "actor": {
      const cx = w / 2;
      const headR = Math.min(w / 3, h / 6);
      const headCy = headR + i;
      const neck = headCy + headR;
      const hip = neck + (h - neck - i) * 0.5;
      const arms = neck + (hip - neck) * 0.3;

      return [
        { kind: "circle", cx: r(cx), cy: r(headCy), r: r(headR) },
        {
          kind: "line",
          d:
            `M ${r(cx)} ${r(neck)} L ${r(cx)} ${r(hip)} ` +
            `M ${r(i)} ${r(arms)} L ${r(w - i)} ${r(arms)} ` +
            `M ${r(cx)} ${r(hip)} L ${r(i)} ${r(h - i)} ` +
            `M ${r(cx)} ${r(hip)} L ${r(w - i)} ${r(h - i)}`,
        },
      ];
    }
  }
}

/**
 * Contorno de una caja de clase: el marco y los dos separadores. Espejo de
 * `class_primitives` en `shape_geometry.rs`.
 *
 * No hace falta saber cuantas operaciones hay: el marco cierra el ultimo
 * compartimento.
 */
export function classPrimitivesFor(
  stereotype: boolean,
  attributes: number,
  w: number,
  h: number,
): Primitive[] {
  const i = INSET;
  const out: Primitive[] = [
    { kind: "rect", x: r(i), y: r(i), width: r(w - i * 2), height: r(h - i * 2) },
  ];

  // Los dos separadores son lo que distingue una clase de un rectangulo con
  // texto dentro, asi que se dibujan siempre que quepan.
  for (const y of [
    classAttributesTop(stereotype),
    classOperationsTop(stereotype, attributes),
  ]) {
    if (y >= h - i) continue;
    out.push({ kind: "line", d: `M ${r(i)} ${r(y)} L ${r(w - i)} ${r(y)}` });
  }

  return out;
}

/**
 * Contorno de una tabla ER: marco, separador de cabecera y una linea entre
 * filas. Espejo de `table_primitives` en `shape_geometry.rs`.
 */
export function tablePrimitivesFor(rows: number, w: number, h: number): Primitive[] {
  const i = INSET;
  const out: Primitive[] = [
    {
      kind: "rect",
      x: r(i),
      y: r(i),
      width: r(w - i * 2),
      height: r(h - i * 2),
      radius: TABLE_CORNER_RADIUS,
    },
  ];

  // Sin filas no hay separador: una tabla vacia es una caja con su nombre.
  if (rows > 0) {
    out.push({
      kind: "line",
      d: `M ${r(i)} ${r(TABLE_HEADER_HEIGHT)} L ${r(w - i)} ${r(TABLE_HEADER_HEIGHT)}`,
    });
  }

  for (let index = 1; index < rows; index++) {
    const y = TABLE_HEADER_HEIGHT + index * TABLE_ROW_HEIGHT;
    if (y >= h - i) break;
    out.push({ kind: "line", d: `M ${r(i)} ${r(y)} L ${r(w - i)} ${r(y)}` });
  }

  return out;
}
