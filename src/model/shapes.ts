import type { NodeShape } from "./generated/NodeShape";

/**
 * Tamanos de arranque de cada forma, en unidades del diagrama.
 *
 * Viven en el lienzo y no en Rust a proposito: el tamano real de un nodo sale
 * de medir su texto, y medir texto es cosa del webview (ver riesgo 1 del plan).
 * Estos valores solo cubren el instante entre crear el nodo y medirlo.
 */
const DEFAULT_SIZE: Record<NodeShape, readonly [number, number]> = {
  rectangle: [160, 60],
  roundedRect: [160, 60],
  ellipse: [140, 70],
  diamond: [120, 80],
  cylinder: [120, 90],
  note: [160, 80],
  package: [220, 160],
  actor: [40, 80],
  // El tamano real de una tabla sale de sus columnas (`@/model/table`); esto
  // es solo la cabecera, para el instante antes de calcularlo.
  hexagon: [170, 70],
  parallelogram: [170, 60],
  table: [220, 28],
  // Igual que la tabla: lo real sale de sus miembros (`@/model/classBox`), y
  // esto es la caja vacia con sus tres compartimentos.
  class: [180, 50],
  // Los extremos de una maquina de estados son discos: cuadrados y
  // pequenos. La barra nace tumbada, que es como mas se usa.
  initialNode: [24, 24],
  finalNode: [28, 28],
  bar: [140, 12],
  // El texto crece con su contenido; esto es el hueco inicial.
  text: [160, 32],
};

export function defaultSizeFor(shape: NodeShape): readonly [number, number] {
  return DEFAULT_SIZE[shape];
}

/** Tamano minimo al que se puede encoger un nodo arrastrando. */
export const MIN_NODE_SIZE = { width: 40, height: 28 } as const;

export const SHAPE_LABEL: Record<NodeShape, string> = {
  rectangle: "Rectangulo",
  roundedRect: "Rectangulo redondeado",
  ellipse: "Elipse",
  diamond: "Rombo",
  cylinder: "Cilindro",
  note: "Nota",
  package: "Paquete",
  actor: "Actor",
  hexagon: "Hexagono",
  parallelogram: "Paralelogramo",
  class: "Clase",
  initialNode: "Inicio",
  finalNode: "Fin",
  bar: "Barra",
  table: "Tabla",
  text: "Texto",
};

/** Formas que pueden contener a otros nodos. */
export function isContainer(shape: NodeShape): boolean {
  return shape === "package";
}

/**
 * Donde cae la etiqueta dentro de la forma. El actor es una figura de palo y
 * su nombre cuelga debajo; el paquete rotula su solapa.
 */
export function labelPlacement(shape: NodeShape): "center" | "below" | "tab" {
  // Una figura de lineas o una barra de seis pixeles no tienen dentro donde
  // poner su nombre. Esconderlo seria peor: en una maquina de estados el
  // nombre de la barra es con el que las transiciones la referencian.
  if (shape === "actor" || shape === "bar") return "below";
  if (shape === "package") return "tab";
  return "center";
}
