import { Handle, NodeResizer, Position, type NodeProps } from "@xyflow/react";
import { memo } from "react";

import type { ErColumn } from "@/model/generated/ErColumn";
import { nodeStyle } from "@/model/style";
import { tablePrimitivesFor } from "@/model/shapeGeometry";
import {
  TABLE_HEADER_HEIGHT,
  TABLE_ROW_HEIGHT,
  columnDetail,
  erTableOf,
  tableSize,
} from "@/model/table";
import type { DiagramFlowNode } from "./adapter";
import "./TableNode.css";

/**
 * Tabla de un diagrama entidad-relacion.
 *
 * El contorno sale de `tablePrimitivesFor`, el mismo que usa el exportador de
 * Rust. El texto se alinea a los bordes —nombre a la izquierda, tipo a la
 * derecha—, asi que no hace falta medirlo para colocarlo: solo para decidir
 * el ancho de la caja, que es lo que hace `tableSize`.
 */
const SIDES = [
  { id: "top", position: Position.Top },
  { id: "right", position: Position.Right },
  { id: "bottom", position: Position.Bottom },
  { id: "left", position: Position.Left },
] as const;

function TableNodeImpl({ data, selected, width, height }: NodeProps<DiagramFlowNode>) {
  const { node } = data;
  const table = erTableOf(node);
  const fallback = tableSize(node);

  const w = width ?? node.geometry?.width ?? fallback.width;
  const h = height ?? node.geometry?.height ?? fallback.height;
  const style = nodeStyle(node);

  const columns = table?.columns ?? [];

  return (
    <>
      <NodeResizer
        isVisible={selected}
        minWidth={120}
        minHeight={TABLE_HEADER_HEIGHT}
        lineClassName="shape-node__resize-line"
        handleClassName="shape-node__resize-handle"
      />

      <div className="table-node">
        <svg
          className="table-node__frame"
          width={w}
          height={h}
          viewBox={`0 0 ${w} ${h}`}
          style={{ "--node-fill": style.fill } as React.CSSProperties}
        >
          {tablePrimitivesFor(columns.length, w, h).map((primitive, index) =>
            primitive.kind === "rect" ? (
              <rect
                key={index}
                x={primitive.x}
                y={primitive.y}
                width={primitive.width}
                height={primitive.height}
                rx={primitive.radius}
                ry={primitive.radius}
                className="table-node__body"
              />
            ) : (
              <path
                key={index}
                d={"d" in primitive ? primitive.d : ""}
                className="table-node__rule"
              />
            ),
          )}
        </svg>

        <div className="table-node__content" style={{ color: style.textColor }}>
          <div className="table-node__header" style={{ height: TABLE_HEADER_HEIGHT }}>
            {node.label}
          </div>

          {columns.map((column) => (
            <Row key={column.id} column={column} />
          ))}
        </div>
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

function Row({ column }: { column: ErColumn }) {
  const detail = columnDetail(column);

  return (
    <div
      className="table-node__row"
      style={{ height: TABLE_ROW_HEIGHT }}
      data-pk={column.primaryKey || undefined}
    >
      <span className="table-node__name">{column.name}</span>
      {detail && <span className="table-node__detail">{detail}</span>}
    </div>
  );
}

export const TableNode = memo(TableNodeImpl);
