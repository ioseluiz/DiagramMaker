import type { NodeShape } from "@/model/generated/NodeShape";

/**
 * Cuerpo de cada forma, dibujado en SVG.
 *
 * Se dibuja en SVG y no con CSS a proposito: son las mismas primitivas
 * (`rect`, `ellipse`, `path`) que emitira el exportador de Rust en F2, asi que
 * lo que se ve en el lienzo y lo que sale exportado parten de la misma
 * geometria en lugar de dos aproximaciones distintas.
 */

const RADIUS = 10;
/** Alto de la solapa del paquete y de la elipse superior del cilindro. */
const TAB_HEIGHT = 22;
const CYLINDER_LIP = 12;

export interface ShapeBodyProps {
  shape: NodeShape;
  width: number;
  height: number;
}

export function ShapeBody({ shape, width: w, height: h }: ShapeBodyProps) {
  // El trazo se dibuja centrado en el borde, asi que medio grosor se sale de la
  // caja. Encogemos medio pixel por lado para que no quede recortado.
  const i = 0.75;

  switch (shape) {
    case "roundedRect":
      return (
        <rect
          x={i}
          y={i}
          width={w - i * 2}
          height={h - i * 2}
          rx={RADIUS}
          ry={RADIUS}
          className="shape-body__fill"
        />
      );

    case "ellipse":
      return (
        <ellipse
          cx={w / 2}
          cy={h / 2}
          rx={w / 2 - i}
          ry={h / 2 - i}
          className="shape-body__fill"
        />
      );

    case "diamond":
      return (
        <path
          d={`M ${w / 2} ${i} L ${w - i} ${h / 2} L ${w / 2} ${h - i} L ${i} ${h / 2} Z`}
          className="shape-body__fill"
        />
      );

    case "cylinder": {
      const ry = Math.min(CYLINDER_LIP, h / 4);
      return (
        <>
          <path
            d={
              `M ${i} ${ry + i} ` +
              `A ${w / 2 - i} ${ry} 0 0 1 ${w - i} ${ry + i} ` +
              `L ${w - i} ${h - ry - i} ` +
              `A ${w / 2 - i} ${ry} 0 0 1 ${i} ${h - ry - i} Z`
            }
            className="shape-body__fill"
          />
          <path
            d={`M ${i} ${ry + i} A ${w / 2 - i} ${ry} 0 0 0 ${w - i} ${ry + i}`}
            className="shape-body__line"
          />
        </>
      );
    }

    case "note": {
      const fold = Math.min(18, w / 4, h / 3);
      return (
        <>
          <path
            d={
              `M ${i} ${i} L ${w - fold - i} ${i} L ${w - i} ${fold + i} ` +
              `L ${w - i} ${h - i} L ${i} ${h - i} Z`
            }
            className="shape-body__fill"
          />
          <path
            d={`M ${w - fold - i} ${i} L ${w - fold - i} ${fold + i} L ${w - i} ${fold + i}`}
            className="shape-body__line"
          />
        </>
      );
    }

    case "package": {
      const tabWidth = Math.min(w * 0.45, 120);
      return (
        <>
          <path
            d={`M ${i} ${TAB_HEIGHT + i} L ${i} ${i} L ${tabWidth} ${i} L ${tabWidth} ${TAB_HEIGHT + i}`}
            className="shape-body__fill"
          />
          <rect
            x={i}
            y={TAB_HEIGHT + i}
            width={w - i * 2}
            height={h - TAB_HEIGHT - i * 2}
            className="shape-body__fill"
          />
        </>
      );
    }

    case "actor": {
      const cx = w / 2;
      const headR = Math.min(w / 3, h / 6);
      const headCy = headR + i;
      const neck = headCy + headR;
      const hip = neck + (h - neck - i) * 0.5;

      return (
        <>
          <circle cx={cx} cy={headCy} r={headR} className="shape-body__fill" />
          <path
            d={
              `M ${cx} ${neck} L ${cx} ${hip} ` +
              `M ${i} ${neck + (hip - neck) * 0.3} L ${w - i} ${neck + (hip - neck) * 0.3} ` +
              `M ${cx} ${hip} L ${i} ${h - i} ` +
              `M ${cx} ${hip} L ${w - i} ${h - i}`
            }
            className="shape-body__line"
          />
        </>
      );
    }

    case "rectangle":
    default:
      return (
        <rect
          x={i}
          y={i}
          width={w - i * 2}
          height={h - i * 2}
          className="shape-body__fill"
        />
      );
  }
}
