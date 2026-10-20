import { Handle, NodeResizer, Position, type NodeProps } from "@xyflow/react";
import { memo } from "react";

import type { ClassMember } from "@/model/generated/ClassMember";
import { classBoxOf, classSize, hasStereotype, memberText } from "@/model/classBox";
import {
  CLASS_HEADER_HEIGHT,
  CLASS_ROW_HEIGHT,
  CLASS_STEREOTYPE_HEIGHT,
  classCompartmentHeight,
} from "@/model/classMetrics";
import { classPrimitivesFor } from "@/model/shapeGeometry";
import { nodeStyle } from "@/model/style";
import type { DiagramFlowNode } from "./adapter";
import "./ClassNode.css";

/**
 * Caja de un diagrama de clases.
 *
 * Mismo reparto de trabajo que la tabla: el contorno sale de
 * `classPrimitivesFor`, el mismo que usa el exportador de Rust, y el texto se
 * coloca con un apilado corriente porque va alineado a la izquierda. Medir
 * solo hace falta para decidir el ancho de la caja.
 */
const SIDES = [
  { id: "top", position: Position.Top },
  { id: "right", position: Position.Right },
  { id: "bottom", position: Position.Bottom },
  { id: "left", position: Position.Left },
] as const;

function ClassNodeImpl({ data, selected, width, height }: NodeProps<DiagramFlowNode>) {
  const { node } = data;
  const box = classBoxOf(node);

  // `classSize` mide texto; solo se llama si de verdad falta algun tamano.
  const knownWidth = width ?? node.geometry?.width;
  const knownHeight = height ?? node.geometry?.height;
  const fallback =
    knownWidth === undefined || knownHeight === undefined ? classSize(node) : null;

  const w = knownWidth ?? fallback!.width;
  const h = knownHeight ?? fallback!.height;
  const style = nodeStyle(node);

  const attributes = box?.attributes ?? [];
  const operations = box?.operations ?? [];
  const stereotype = box ? hasStereotype(box) : false;

  return (
    <>
      <NodeResizer
        isVisible={selected}
        minWidth={120}
        minHeight={CLASS_HEADER_HEIGHT}
        lineClassName="shape-node__resize-line"
        handleClassName="shape-node__resize-handle"
      />

      <div className="class-node">
        <svg
          className="class-node__frame"
          width={w}
          height={h}
          viewBox={`0 0 ${w} ${h}`}
          style={{ "--node-fill": style.fill } as React.CSSProperties}
        >
          {classPrimitivesFor(stereotype, attributes.length, w, h).map(
            (primitive, index) =>
              primitive.kind === "rect" ? (
                <rect
                  key={index}
                  x={primitive.x}
                  y={primitive.y}
                  width={primitive.width}
                  height={primitive.height}
                  className="class-node__body"
                />
              ) : (
                <path
                  key={index}
                  d={"d" in primitive ? primitive.d : ""}
                  className="class-node__rule"
                />
              ),
          )}
        </svg>

        <div className="class-node__content" style={{ color: style.textColor }}>
          <div
            className="class-node__header"
            style={{
              height: stereotype
                ? CLASS_HEADER_HEIGHT + CLASS_STEREOTYPE_HEIGHT
                : CLASS_HEADER_HEIGHT,
            }}
          >
            {stereotype && (
              <span className="class-node__stereotype">
                «{box!.stereotype!.trim()}»
              </span>
            )}
            <span className="class-node__name">{node.label}</span>
          </div>

          <Compartment members={attributes} isOperation={false} />
          <Compartment members={operations} isOperation />
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

function Compartment({
  members,
  isOperation,
}: {
  members: ClassMember[];
  isOperation: boolean;
}) {
  return (
    <div
      className="class-node__compartment"
      style={{ minHeight: classCompartmentHeight(members.length) }}
    >
      {members.map((member) => (
        <div
          key={member.id}
          className="class-node__member"
          style={{ height: CLASS_ROW_HEIGHT }}
          data-static={member.isStatic || undefined}
          data-abstract={member.isAbstract || undefined}
        >
          {memberText(member, isOperation)}
        </div>
      ))}
    </div>
  );
}

export const ClassNode = memo(ClassNodeImpl);
