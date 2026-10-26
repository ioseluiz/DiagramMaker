/**
 * Colores de relleno para nodos y tablas.
 *
 * Son tintes translucidos, no colores solidos, y eso es deliberado: un relleno
 * con alfa se compone sobre el fondo que haya debajo, asi que el mismo valor
 * da un azul palido sobre fondo claro y un azul profundo sobre fondo oscuro.
 * El texto puede seguir siendo el del tema en ambos casos.
 *
 * Con colores solidos haria falta calcular un color de texto que contraste y
 * mantener ese calculo igual en el lienzo y en el exportador de Rust. Esto
 * evita el problema entero.
 */

export interface Swatch {
  /** Nombre para el `title` del boton. */
  name: string;
  /** `null` = sin color, usa el del tema. */
  value: string | null;
}

/** Opacidad de los tintes. Suficiente para distinguir, sin tapar el texto. */
const ALPHA = 0.16;

function tint(r: number, g: number, b: number): string {
  // La forma clasica con comas, no `rgb(r g b / a)` de CSS Color 4. El
  // navegador entiende las dos; el exportador solo esta, y lo que no
  // entiende lo pinta negro. Ver `src-tauri/src/render/color.rs`.
  return `rgba(${r}, ${g}, ${b}, ${ALPHA})`;
}

export const SWATCHES: readonly Swatch[] = [
  { name: "Sin color", value: null },
  { name: "Azul", value: tint(37, 99, 235) },
  { name: "Verde", value: tint(21, 128, 61) },
  { name: "Ambar", value: tint(217, 119, 6) },
  { name: "Rojo", value: tint(194, 52, 29) },
  { name: "Morado", value: tint(124, 58, 237) },
  { name: "Turquesa", value: tint(13, 148, 136) },
  { name: "Gris", value: tint(100, 116, 139) },
];

/** Color con el que pintar la muestra del selector, ya opaco para que se vea. */
export function swatchPreview(value: string | null): string {
  if (!value) return "transparent";
  // La misma tinta al triple de opacidad: a 0.16 la muestra seria invisible.
  return value.replace(`/ ${ALPHA}`, "/ 0.55");
}
