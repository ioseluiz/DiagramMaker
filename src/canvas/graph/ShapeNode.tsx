import { Handle, NodeResizer, Position, type NodeProps } from "@xyflow/react";
import { memo, useMemo } from "react";

import { MIN_NODE_SIZE, defaultSizeFor, labelPlacement } from "@/model/shapes";
import { nodeStyle } from "@/model/style";
import { fontSizeFor, labelMaxWidth, lineHeightFor, wrapText } from "@/canvas/text";
import { ShapeBody } from "./ShapeBody";
import type { DiagramFlowNode } from "./adapter";
import "./ShapeNode.css";

/**
 * Los cuatro lados actuan a la vez como origen y destino de conexion: con
 * `ConnectionMode.Loose` en el lienzo, React Flow acepta unir dos `source`.
 * Asi se tira una arista desde cualquier borde sin decidir antes el sentido,
 * que es como se dibuja en una herramienta de diagramas.
 *
 * Los ids son literalmente los valores de `Side`, de modo que lo que React
 * Flow devuelve en `onConnect` se guarda en el documento sin traducir.
 */
const SIDES = [
  { id: "top", position: Position.Top },
  { id: "right", position: Position.Right },
  { id: "bottom", position: Position.Bottom },
  { id: "left", position: Position.Left },
] as const;

function ShapeNodeImpl({ data, selected, width, height }: NodeProps<DiagramFlowNode>) {
  const { node, label } = data;
  const [defaultWidth, defaultHeight] = defaultSizeFor(node.shape);
  const w = width ?? node.geometry?.width ?? defaultWidth;
  const h = height ?? node.geometry?.height ?? defaultHeight;

  const style = nodeStyle(node);
  const placement = labelPlacement(node.shape);
  const fontSize = style.fontSize ?? fontSizeFor(node.shape);

  // El texto se parte aqui, no con CSS: son las mismas lineas que recibe el
  // exportador, asi que el SVG exportado rompe por donde rompe el lienzo.
  const lines = useMemo(
    () =>
      wrapText(label, {
        maxWidth: labelMaxWidth(node.shape, w),
        size: fontSize,
        bold: style.bold,
        italic: style.italic,
      }),
    [label, node.shape, w, fontSize, style.bold, style.italic],
  );

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
            fontSize,
            lineHeight: `${lineHeightFor(fontSize)}px`,
            fontWeight: style.bold ? 600 : undefined,
            fontStyle: style.italic ? "italic" : undefined,
          }}
        >
          {lines.map((line, index) => (
            <span key={index} className="shape-node__line">
              {line.text}
            </span>
          ))}
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
