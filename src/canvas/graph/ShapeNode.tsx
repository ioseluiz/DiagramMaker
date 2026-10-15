import { Handle, NodeResizer, Position, type NodeProps } from "@xyflow/react";
import { memo } from "react";

import { MIN_NODE_SIZE, defaultSizeFor, labelPlacement } from "@/model/shapes";
import { nodeStyle } from "@/model/style";
import { ShapeBody } from "./ShapeBody";
import type { DiagramFlowNode } from "./adapter";
import "./ShapeNode.css";

/**
 * Los cuatro lados actuan a la vez como origen y destino de conexion: con
 * `ConnectionMode.Loose` en el lienzo, React Flow acepta unir dos `source`.
 * Asi se tira una arista desde cualquier borde sin decidir antes el sentido,
 * que es como se dibuja en una herramienta de diagramas.
 */
const SIDES = [
  { id: "t", position: Position.Top },
  { id: "r", position: Position.Right },
  { id: "b", position: Position.Bottom },
  { id: "l", position: Position.Left },
] as const;

function ShapeNodeImpl({ data, selected, width, height }: NodeProps<DiagramFlowNode>) {
  const { node, label } = data;
  const [defaultWidth, defaultHeight] = defaultSizeFor(node.shape);
  const w = width ?? node.geometry?.width ?? defaultWidth;
  const h = height ?? node.geometry?.height ?? defaultHeight;

  const style = nodeStyle(node);
  const placement = labelPlacement(node.shape);

  return (
    <>
      <NodeResizer
        isVisible={selected}
        minWidth={MIN_NODE_SIZE.width}
        minHeight={MIN_NODE_SIZE.height}
        lineClassName="shape-node__resize-line"
        handleClassName="shape-node__resize-handle"
      />

      <div className="shape-node" data-selected={selected || undefined}>
        <svg
          className="shape-node__body"
          width={w}
          height={h}
          viewBox={`0 0 ${w} ${h}`}
          style={
            {
              "--node-fill": style.fill,
              "--node-stroke": style.stroke,
              "--node-stroke-width": style.strokeWidth,
              "--node-dash": style.dashed ? "6 4" : undefined,
            } as React.CSSProperties
          }
        >
          <ShapeBody shape={node.shape} width={w} height={h} />
        </svg>

        <span
          className="shape-node__label"
          data-placement={placement}
          style={{
            color: style.textColor,
            fontSize: style.fontSize,
            fontWeight: style.bold ? 600 : undefined,
            fontStyle: style.italic ? "italic" : undefined,
          }}
        >
          {label}
        </span>
      </div>

      {SIDES.map((side) => (
        <Handle
          key={side.id}
          id={side.id}
          type="source"
          position={side.position}
          className="shape-node__handle"
        />
      ))}
    </>
  );
}

export const ShapeNode = memo(ShapeNodeImpl);
