/**
 * Formato de tiempo relativo en el idioma del usuario.
 *
 * «hace 12 min» cabe en un aviso pequeno; «15/10/2026, 9:27:58» no, y ademas
 * obliga a hacer la resta mentalmente para saber si esa copia importa.
 */

const DIVISIONS: { limit: number; unit: Intl.RelativeTimeFormatUnit; ms: number }[] = [
  { limit: 60_000, unit: "second", ms: 1_000 },
  { limit: 3_600_000, unit: "minute", ms: 60_000 },
  { limit: 86_400_000, unit: "hour", ms: 3_600_000 },
  { limit: 2_592_000_000, unit: "day", ms: 86_400_000 },
  { limit: Number.POSITIVE_INFINITY, unit: "month", ms: 2_592_000_000 },
];

/**
 * Idioma de la interfaz.
 *
 * Fijo, no el del sistema: toda la aplicacion esta en espanol, y dejar que
 * `Intl` siguiera al sistema producia «13 min. ago» dentro de un aviso que
 * dice «Cambios sin guardar». El dia que la interfaz se traduzca, este es el
 * sitio donde conectarlo.
 */
export const UI_LOCALE = "es";

let formatter: Intl.RelativeTimeFormat | null = null;

function relativeFormatter(): Intl.RelativeTimeFormat {
  formatter ??= new Intl.RelativeTimeFormat(UI_LOCALE, {
    // `auto` prefiere «ayer» a «hace 1 dia» cuando el idioma tiene la palabra.
    numeric: "auto",
    style: "short",
  });
  return formatter;
}

/**
 * Describe `timestamp` respecto a `now`. Ambos en milisegundos desde el epoch.
 */
export function relativeTime(timestamp: number, now: number = Date.now()): string {
  const diff = timestamp - now;
  const magnitude = Math.abs(diff);

  const division =
    DIVISIONS.find((d) => magnitude < d.limit) ?? DIVISIONS[DIVISIONS.length - 1]!;

  return relativeFormatter().format(Math.round(diff / division.ms), division.unit);
}

/** Nombre del archivo a partir de su ruta completa. */
export function basename(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}
