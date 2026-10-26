import { SWATCHES } from "@/model/palette";

/**
 * Selector de relleno.
 *
 * Una fila de muestras en vez de un `<input type="color">`: el nativo abre un
 * dialogo del sistema, no sabe expresar "sin color" y deja elegir cualquier
 * tono, con lo que dos diagramas del mismo proyecto acaban sin parecerse.
 */
export interface ColorPickerProps {
  value: string | null;
  onChange: (value: string | null) => void;
}

export function ColorPicker({ value, onChange }: ColorPickerProps) {
  return (
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
  );
}
