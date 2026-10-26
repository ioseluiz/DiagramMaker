/**
 * Colores de relleno para nodos y tablas.
 *
 * Son tintes translucidos, no colores solidos, y eso es deliberado: un relleno
 * con alfa se compone sobre el fondo que haya debajo, asi que el mismo valor
 * da un azul palido sobre fondo claro y un azul profundo sobre fondo oscuro.
 * El texto puede seguir siendo el del tema en ambos casos.
 */

export interface Swatch {
  /** Nombre para el `title` del boton. */
  name: string;
  /** `null` = sin color, usa el del tema. */
  value: string | null;
  /** Color con el que se pinta el boton del selector. */
  preview: string;
}

/** Opacidad de los tintes. Suficiente para distinguir, sin tapar el texto. */
export const ALPHA = 0.16;
/** La del boton del selector: a 0.16 la muestra seria invisible. */
const PREVIEW_ALPHA = 0.55;

/**
 * Un color con la opacidad dada, en la forma clasica con comas.
 *
 * No `rgb(r g b / a)` de CSS Color 4: el navegador entiende las dos, pero el
 * exportador solo esta, y lo que no entiende lo pinta negro. Ver
 * `src-tauri/src/render/color.rs`.
 */
export function rgba(r: number, g: number, b: number, alpha: number): string {
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/**
 * Los tintes, por sus componentes.
 *
 * El valor guardado y el del boton salen los dos de aqui. Antes el boton se
 * sacaba buscando y sustituyendo el alfa dentro del texto del color ya
 * montado, y eso se rompio en silencio en cuanto cambio la forma de
 * escribirlo: las muestras se quedaron al 16 % y casi no se veian. Generar
 * los dos de la misma fuente quita esa clase de fallo, no solo ese caso.
 */
const TINTS: readonly (readonly [string, number, number, number])[] = [
  ["Azul", 37, 99, 235],
  ["Verde", 21, 128, 61],
  ["Ambar", 217, 119, 6],
  ["Rojo", 194, 52, 29],
  ["Morado", 124, 58, 237],
  ["Turquesa", 13, 148, 136],
  ["Gris", 100, 116, 139],
];

export const SWATCHES: readonly Swatch[] = [
  { name: "Sin color", value: null, preview: "transparent" },
  ...TINTS.map(([name, r, g, b]) => ({
    name,
    value: rgba(r, g, b, ALPHA),
    preview: rgba(r, g, b, PREVIEW_ALPHA),
  })),
];
