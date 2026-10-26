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

/* --- Color libre ---------------------------------------------------------- */

/** Un relleno descompuesto en lo que el selector necesita manejar. */
export interface Fill {
  r: number;
  g: number;
  b: number;
  /** De 0 a 1. */
  alpha: number;
}

/** Intensidad minima que ofrece el deslizador, en centesimas. */
export const MIN_STRENGTH = 5;

/**
 * Lee un relleno guardado.
 *
 * Acepta las formas que pueden acabar en un documento: las que escribe esta
 * paleta, las que trae un importador y las que alguien teclee a mano en el
 * `.dgm`. Devuelve `null` para lo que no sepa leer —un degradado, un nombre
 * de color CSS— y entonces el selector se queda en su valor por defecto en
 * lugar de inventarse uno.
 */
export function parseFill(value: string | null | undefined): Fill | null {
  if (!value) return null;
  const text = value.trim();

  const hex = parseHex(text);
  if (hex) return hex;

  const abre = text.indexOf("(");
  if (abre < 0) return null;
  if (!/^rgba?$/i.test(text.slice(0, abre).trim())) return null;

  const partes = text
    .slice(abre + 1)
    .replace(/\)\s*$/, "")
    // Tanto `r, g, b, a` como `r g b / a`: la segunda no la entiende el
    // exportador, pero si puede estar escrita en un documento viejo.
    .split(/[,/\s]+/)
    .filter(Boolean)
    .map(Number);

  if (partes.length < 3 || partes.slice(0, 3).some((n) => !Number.isFinite(n))) {
    return null;
  }

  const alpha = partes.length > 3 && Number.isFinite(partes[3]!) ? partes[3]! : 1;
  return {
    r: clampByte(partes[0]!),
    g: clampByte(partes[1]!),
    b: clampByte(partes[2]!),
    alpha: Math.min(Math.max(alpha, 0), 1),
  };
}

/** Lee `#rgb`, `#rrggbb` o `#rrggbbaa`, con o sin almohadilla. */
export function parseHex(value: string): Fill | null {
  const crudo = value.trim().replace(/^#/, "");
  if (!/^[0-9a-f]+$/i.test(crudo)) return null;

  const byte = (at: number, len: number) =>
    parseInt(len === 1 ? crudo[at]!.repeat(2) : crudo.slice(at * 2, at * 2 + 2), 16);

  if (crudo.length === 3) {
    return { r: byte(0, 1), g: byte(1, 1), b: byte(2, 1), alpha: 1 };
  }
  if (crudo.length === 6 || crudo.length === 8) {
    const alpha = crudo.length === 8 ? parseInt(crudo.slice(6, 8), 16) / 255 : 1;
    return { r: byte(0, 2), g: byte(1, 2), b: byte(2, 2), alpha };
  }
  return null;
}

/** `#rrggbb` del color, sin su opacidad. */
export function toHex({ r, g, b }: Fill): string {
  return `#${[r, g, b].map((n) => clampByte(n).toString(16).padStart(2, "0")).join("")}`;
}

function clampByte(n: number): number {
  return Math.min(Math.max(Math.round(n), 0), 255);
}

/** Los dos colores de texto entre los que se elige. */
const LIGHT_TEXT = "#ffffff";
const DARK_TEXT = "#16191d";

/**
 * Color de texto que se lee sobre este relleno, o `null` si vale el del tema.
 *
 * El relleno puede ser translucido, asi que lo que se ve es el resultado de
 * componerlo sobre el fondo del lienzo — y ese fondo cambia con el tema,
 * mientras que el color que se guarda aqui es uno solo para los dos.
 *
 * La regla es una comparacion, no un umbral: **se fija un color solo cuando
 * se lee mejor que lo que daria el tema.** No fijar nada no es neutro, es
 * elegir la pareja del tema —texto oscuro sobre fondo claro, texto claro
 * sobre fondo oscuro—, que sobre un tinte palido es inmejorable y sobre un
 * relleno opaco es una moneda al aire. Comparar con ella cubre los dos
 * extremos y todo lo que hay en medio con una sola frase.
 *
 * Un umbral fijo no vale: con solo dos colores de texto, un relleno opaco de
 * tono medio no llega a 4,5 a 1 con ninguno de los dos, y rechazarlos por eso
 * lo dejaba en manos del tema, que ahi lo hace peor.
 *
 * Los fondos con los que se prueba son el blanco y el negro puros, mas
 * extremos que los reales: pasarse de prudente deja el color del tema,
 * quedarse corto deja texto ilegible.
 */
export function textColorOn(fill: Fill): string | null {
  const sobreClaro = luminance(composite(fill, 255));
  const sobreOscuro = luminance(composite(fill, 0));

  const lumClaro = luminance(parseHex(LIGHT_TEXT)!);
  const lumOscuro = luminance(parseHex(DARK_TEXT)!);

  // Lo que se lee si no se fija nada, en el peor de los dos temas.
  const delTema = Math.min(
    contrast(lumOscuro, sobreClaro),
    contrast(lumClaro, sobreOscuro),
  );

  let mejor: string | null = null;
  let peorDelMejor = 0;

  for (const [texto, lum] of [
    [LIGHT_TEXT, lumClaro],
    [DARK_TEXT, lumOscuro],
  ] as const) {
    const peor = Math.min(contrast(lum, sobreClaro), contrast(lum, sobreOscuro));

    if (peor > peorDelMejor) {
      peorDelMejor = peor;
      mejor = texto;
    }
  }

  return peorDelMejor > delTema ? mejor : null;
}

/** Razon de contraste entre dos luminancias, de 1 a 21. Formula de la WCAG. */
function contrast(a: number, b: number): number {
  const [alta, baja] = a > b ? [a, b] : [b, a];
  return (alta + 0.05) / (baja + 0.05);
}

/** El relleno compuesto sobre un fondo gris del nivel dado. */
function composite({ r, g, b, alpha }: Fill, background: number): Fill {
  const mix = (c: number) => c * alpha + background * (1 - alpha);
  return { r: mix(r), g: mix(g), b: mix(b), alpha: 1 };
}

/** Luminancia relativa, de 0 (negro) a 1 (blanco). Formula de la WCAG. */
function luminance({ r, g, b }: Fill): number {
  const canal = (v: number) => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * canal(r) + 0.7152 * canal(g) + 0.0722 * canal(b);
}
