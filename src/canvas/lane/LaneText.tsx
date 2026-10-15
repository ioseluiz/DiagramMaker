import type { LabelLayout } from "@/model/generated/LabelLayout";

/**
 * Un bloque de texto ya medido, dibujado como lo dibuja Rust.
 *
 * La linea base se calcula a mano —`0.35 * fontSize` por debajo del centro de
 * la linea— porque `dominant-baseline` no da lo mismo en todos los visores.
 * Es la misma cuenta que hace `text_block` en `render/svg.rs`: si una de las
 * dos cambia, el texto del PNG deja de caer donde cae en pantalla.
 */
export interface LaneTextProps {
  label: LabelLayout | undefined;
  /** Punto de anclaje horizontal, segun `anchor`. */
  x: number;
  /** Borde superior del bloque. */
  y: number;
  anchor?: "start" | "middle";
  className?: string;
  color?: string;
}

export function LaneText({
  label,
  x,
  y,
  anchor = "middle",
  className,
  color,
}: LaneTextProps) {
  if (!label || label.lines.length === 0) return null;

  return (
    <text
      className={className}
      textAnchor={anchor}
      fontSize={label.fontSize}
      style={{ fill: color }}
    >
      {label.lines.map((line, index) => (
        <tspan
          key={index}
          x={x}
          y={y + label.lineHeight * (index + 0.5) + label.fontSize * 0.35}
        >
          {line.text}
        </tspan>
      ))}
    </text>
  );
}
