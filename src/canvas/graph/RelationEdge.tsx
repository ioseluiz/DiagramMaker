import {
  BaseEdge,
  EdgeLabelRenderer,
  Position,
  getBezierPath,
  getSmoothStepPath,
  getStraightPath,
  type EdgeProps,
} from "@xyflow/react";
import { memo } from "react";

import type { Routing } from "@/model/generated/Routing";
import "./RelationEdge.css";

/**
 * Arista del diagrama.
 *
 * Existe para dibujar las cardinalidades pegadas a cada extremo. React Flow
 * solo sabe poner una etiqueta en el centro, y en un entidad-relacion el
 * `1` y el `*` de los extremos son justo la informacion que se busca de un
 * vistazo: sin ellos el diagrama dice que dos tablas se relacionan, pero no
 * como.
 *
 * El trazado lo sigue calculando React Flow y sale por `BaseEdge`, que lo
 * pinta como `path.react-flow__edge-path`. La exportacion lee ese mismo
 * atributo del DOM, asi que el SVG exportado usa la curva exacta que se ve.
 */

/** Cuanto se separa la cardinalidad del extremo de la linea. */
const OFFSET = 14;

export interface RelationEdgeData extends Record<string, unknown> {
  routing: Routing;
  sourceLabel?: string;
  targetLabel?: string;
  centerLabel?: string;
}

/** Trazado segun el ruteo que pida la arista. */
function pathFor(
  routing: Routing,
  geometry: Parameters<typeof getSmoothStepPath>[0],
): [string, number, number] {
  switch (routing) {
    case "straight": {
      const [path, x, y] = getStraightPath(geometry);
      return [path, x, y];
    }
    case "bezier": {
      const [path, x, y] = getBezierPath(geometry);
      return [path, x, y];
    }
    case "orthogonal": {
      const [path, x, y] = getSmoothStepPath({ ...geometry, borderRadius: 8 });
      return [path, x, y];
    }
  }
}

function RelationEdgeImpl({
  id,
  sourceX,
  sourceY,
  targetX,
  targetY,
  sourcePosition,
  targetPosition,
  markerStart,
  markerEnd,
  style,
  data,
  selected,
}: EdgeProps) {
  const geometry = {
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
  };

  const {
    routing = "orthogonal",
    sourceLabel,
    targetLabel,
    centerLabel,
  } = (data ?? {}) as RelationEdgeData;

  const [path, centerX, centerY] = pathFor(routing, geometry);

  // El color de trazo llega como estilo en linea desde el adaptador, y eso
  // gana a cualquier regla CSS: la seleccion tiene que resolverse aqui o no
  // se ve. Sin esto se puede seleccionar una relacion y borrarla, pero nada
  // indica cual esta seleccionada.
  const strokeStyle = selected
    ? {
        ...style,
        stroke: "var(--accent)",
        strokeWidth: Math.max(Number(style?.strokeWidth ?? 1.5), 2),
      }
    : style;

  return (
    <>
      <BaseEdge
        id={id}
        path={path}
        markerStart={markerStart}
        markerEnd={markerEnd}
        style={strokeStyle}
      />

      <EdgeLabelRenderer>
        {centerLabel && (
          <div
            className="relation-edge__label"
            data-selected={selected || undefined}
            style={{
              transform: `translate(-50%, -50%) translate(${centerX}px, ${centerY}px)`,
            }}
          >
            {centerLabel}
          </div>
        )}

        {sourceLabel && (
          <Cardinality
            text={sourceLabel}
            x={sourceX}
            y={sourceY}
            position={sourcePosition}
            selected={selected}
          />
        )}

        {targetLabel && (
          <Cardinality
            text={targetLabel}
            x={targetX}
            y={targetY}
            position={targetPosition}
            selected={selected}
          />
        )}
      </EdgeLabelRenderer>
    </>
  );
}

interface CardinalityProps {
  text: string;
  x: number;
  y: number;
  position: Position;
  selected?: boolean;
}

/** Cardinalidad de un extremo, apartada hacia fuera del nodo. */
function Cardinality({ text, x, y, position, selected }: CardinalityProps) {
  // Se separa en la direccion en la que sale la linea, para no caer encima
  // del borde de la caja ni de la propia arista.
  const dx =
    position === Position.Left ? -OFFSET : position === Position.Right ? OFFSET : 0;
  const dy =
    position === Position.Top ? -OFFSET : position === Position.Bottom ? OFFSET : 0;

  return (
    <div
      className="relation-edge__cardinality"
      data-selected={selected || undefined}
      style={{ transform: `translate(-50%, -50%) translate(${x + dx}px, ${y + dy}px)` }}
    >
      {text}
    </div>
  );
}

export const RelationEdge = memo(RelationEdgeImpl);
