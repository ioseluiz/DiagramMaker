import { useId } from "react";

import {
  ALPHA,
  MIN_STRENGTH,
  SWATCHES,
  parseFill,
  parseHex,
  rgba,
  toHex,
  type Fill,
} from "@/model/palette";

/**
 * Selector de relleno: ocho muestras y, debajo, color libre.
 *
 * Las muestras cubren el caso de siempre —marcar cuatro grupos de un vistazo
 * sin pensar en colores— y el color libre el de tener que ceñirse a una
 * paleta ajena. No es un `<input type="color">` a secas porque ese no sabe
 * expresar "sin color" ni la opacidad, que aqui es la mitad del asunto: el
 * mismo tono al 15 % es un fondo y al 100 % es un bloque.
 */
export interface ColorPickerProps {
  value: string | null;
  onChange: (value: string | null) => void;
}

/** Con lo que arranca el color libre cuando el nodo no tiene relleno. */
const DEFAULT_FILL: Fill = { r: 37, g: 99, b: 235, alpha: ALPHA };

export function ColorPicker({ value, onChange }: ColorPickerProps) {
  const hexId = useId();
  const fuerzaId = useId();

  // Lo que haya puesto el nodo manda sobre el valor de arranque, de modo que
  // tocar el deslizador despues de elegir una muestra parte de esa muestra y
  // no de otro color.
  const actual = parseFill(value) ?? DEFAULT_FILL;
  const fuerza = Math.round(actual.alpha * 100);

  const aplicar = (fill: Fill, alpha: number) =>
    onChange(rgba(fill.r, fill.g, fill.b, Math.round(alpha) / 100));

  return (
    <div className="picker">
      <div className="swatches" role="radiogroup" aria-label="Color de relleno">
        {SWATCHES.map((swatch) => {
          const active = (value ?? null) === swatch.value;

          return (
            <button
              key={swatch.name}
              type="button"
              role="radio"
              aria-checked={active}
              aria-label={swatch.name}
              title={swatch.name}
              className="swatches__item"
              data-empty={swatch.value === null || undefined}
              style={{ background: swatch.preview }}
              onClick={() => onChange(swatch.value)}
            />
          );
        })}
      </div>

      <div className="picker__row">
        <label className="picker__name" htmlFor={hexId}>
          Hex
        </label>
        <input
          id={hexId}
          className="picker__hex"
          value={toHex(actual)}
          spellCheck={false}
          // Se escribe libremente y solo se aplica lo que se entiende: con
          // `#2` a medio teclear no tiene sentido repintar el nodo de negro.
          onChange={(e) => {
            const leido = parseHex(e.target.value);
            if (leido) aplicar(leido, fuerza);
          }}
        />
        <input
          type="color"
          className="picker__well"
          aria-label="Elegir un color"
          value={toHex(actual)}
          onChange={(e) => {
            const leido = parseHex(e.target.value);
            if (leido) aplicar(leido, fuerza);
          }}
        />
      </div>

      <div className="picker__row">
        <label className="picker__name" htmlFor={fuerzaId}>
          Fuerza
        </label>
        <input
          id={fuerzaId}
          type="range"
          className="picker__range"
          min={MIN_STRENGTH}
          max={100}
          step={5}
          value={fuerza}
          onChange={(e) => aplicar(actual, Number(e.target.value))}
        />
        <span className="picker__value">{fuerza}%</span>
      </div>
    </div>
  );
}
