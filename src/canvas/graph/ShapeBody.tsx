import type { NodeShape } from "@/model/generated/NodeShape";
import { isFilled, primitivesFor, type Primitive } from "@/model/shapeGeometry";

/**
 * Cuerpo de una forma.
 *
 * No calcula nada: traduce a elementos SVG las primitivas de
 * `@/model/shapeGeometry`, que son exactamente las mismas que usa el
 * exportador de Rust. Por eso el lienzo y el SVG exportado no pueden
 * divergir.
 */
export interface ShapeBodyProps {
  shape: NodeShape;
  width: number;
  height: number;
}

export function ShapeBody({ shape, width, height }: ShapeBodyProps) {
  return (
    <>
      {primitivesFor(shape, width, height).map((primitive, index) => (
        <PrimitiveShape key={index} primitive={primitive} />
      ))}
    </>
  );
}

function PrimitiveShape({ primitive }: { primitive: Primitive }) {
  const className = isFilled(primitive) ? "shape-body__fill" : "shape-body__line";

  switch (primitive.kind) {
    case "rect":
      return (
        <rect
          x={primitive.x}
          y={primitive.y}
          width={primitive.width}
          height={primitive.height}
          rx={primitive.radius}
          ry={primitive.radius}
          className={className}
        />
      );

    case "ellipse":
      return (
        <ellipse
          cx={primitive.cx}
          cy={primitive.cy}
          rx={primitive.rx}
          ry={primitive.ry}
          className={className}
        />
      );

    case "circle":
      return (
        <circle
          cx={primitive.cx}
          cy={primitive.cy}
          r={primitive.r}
          className={className}
        />
      );

    case "path":
    case "line":
      return <path d={primitive.d} className={className} />;
  }
}
