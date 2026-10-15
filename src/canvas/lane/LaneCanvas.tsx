import { useCallback, useMemo, useRef, useState } from "react";

import type { Document } from "@/model/generated/Document";
import { SELF_LOOP_WIDTH, layout } from "@/model/laneGeometry";
import { nodeStyle } from "@/model/style";
import { laneLabels, laneMeasures } from "./measure";
import { useDocumentStore } from "@/store/documentStore";
import { LaneText } from "./LaneText";
import "./LaneCanvas.css";

/**
 * Lienzo de los diagramas de secuencia.
 *
 * No usa React Flow, y no es por capricho: React Flow coloca cajas que el
 * usuario arrastra, y aqui no hay nada que arrastrar. La posicion de todo
 * sale del orden del guion, asi que el lienzo es un SVG que dibuja el reparto
 * y deja elegir piezas. Para mover un mensaje se cambia su sitio en la lista,
 * no su coordenada.
 *
 * El reparto es el mismo que usa la exportacion —`laneGeometry`, espejo del
 * de Rust— de modo que lo que se ve es lo que sale en el PNG.
 */

/** Cuanto acerca o aleja una muesca de la rueda. */
const ZOOM_STEP = 1.12;
const MIN_ZOOM = 0.2;
const MAX_ZOOM = 4;

/** Margen alrededor del diagrama al encuadrarlo. */
const FIT_PADDING = 40;

/** Grosor de la banda invisible que hace mas facil acertarle a una flecha. */
const HIT_WIDTH = 14;

/**
 * Encuadre actual, como `translate(x, y) scale(zoom)` en pixeles de pantalla.
 *
 * Se aplica a un grupo y no al `viewBox`, que obligaria a conocer el tamano
 * del contenedor en cada pintado —y eso es leer el DOM durante el render—.
 */
interface View {
  x: number;
  y: number;
  zoom: number;
}

export interface LaneCanvasProps {
  document: Document;
}

export function LaneCanvas({ document: doc }: LaneCanvasProps) {
  const selection = useDocumentStore((s) => s.selection);
  const select = useDocumentStore((s) => s.select);

  const svgRef = useRef<SVGSVGElement>(null);
  const [view, setView] = useState<View>({ x: 0, y: 0, zoom: 1 });
  const panning = useRef<{ x: number; y: number; view: View } | null>(null);

  // Medir y repartir solo cuando el documento cambia: en un guion largo esto
  // recorre todos los pasos y no debe correr en cada repintado.
  const { placed, labels } = useMemo(() => {
    const measures = laneMeasures(doc);
    const participants = doc.nodes.map((node) => node.id);
    return {
      placed: layout(participants, doc.script ?? { steps: [] }, measures),
      labels: laneLabels(doc),
    };
  }, [doc]);

  const chosen = useMemo(() => new Set(selection), [selection]);

  const fit = useCallback(() => {
    const box = svgRef.current?.getBoundingClientRect();
    if (!box || placed.size.width === 0) return;

    const zoom = Math.min(
      (box.width - FIT_PADDING) / placed.size.width,
      (box.height - FIT_PADDING) / placed.size.height,
      1,
    );
    const safe = Math.max(zoom, MIN_ZOOM);

    setView({
      x: (box.width - placed.size.width * safe) / 2,
      y: (box.height - placed.size.height * safe) / 2,
      zoom: safe,
    });
  }, [placed.size.height, placed.size.width]);

  function onWheel(event: React.WheelEvent<SVGSVGElement>) {
    const box = svgRef.current?.getBoundingClientRect();
    if (!box) return;

    const factor = event.deltaY < 0 ? ZOOM_STEP : 1 / ZOOM_STEP;
    const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, view.zoom * factor));
    if (zoom === view.zoom) return;

    // Acercar hacia el puntero: el punto del diagrama que hay debajo del
    // raton tiene que seguir debajo del raton despues de acercar.
    const px = event.clientX - box.left;
    const py = event.clientY - box.top;
    const ratio = zoom / view.zoom;

    setView({
      x: px - (px - view.x) * ratio,
      y: py - (py - view.y) * ratio,
      zoom,
    });
  }

  function onPointerDown(event: React.PointerEvent<SVGSVGElement>) {
    // Arrastrar sobre el fondo desplaza; sobre una pieza, no.
    if (event.button !== 0 && event.button !== 1) return;
    panning.current = { x: event.clientX, y: event.clientY, view };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function onPointerMove(event: React.PointerEvent<SVGSVGElement>) {
    const start = panning.current;
    if (!start) return;

    setView({
      x: start.view.x + (event.clientX - start.x),
      y: start.view.y + (event.clientY - start.y),
      zoom: start.view.zoom,
    });
  }

  function endPan(event: React.PointerEvent<SVGSVGElement>) {
    panning.current = null;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  /** Elegir una pieza sin que el clic llegue al fondo y la deseleccione. */
  const pick = (id: string) => (event: React.MouseEvent) => {
    event.stopPropagation();
    select(event.shiftKey ? [...new Set([...selection, id])] : [id]);
  };

  return (
    <div className="lane-canvas">
      <svg
        ref={svgRef}
        className="lane-canvas__surface"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={endPan}
        onPointerCancel={endPan}
        onClick={() => select([])}
        role="presentation"
      >
        <defs>
          <marker
            id="lane-arrow-filled"
            viewBox="0 0 12 12"
            markerWidth="11"
            markerHeight="11"
            refX="10"
            refY="6"
            orient="auto-start-reverse"
            markerUnits="userSpaceOnUse"
          >
            <path d="M 1 1 L 11 6 L 1 11 z" className="lane-marker" />
          </marker>
          <marker
            id="lane-arrow-open"
            viewBox="0 0 12 12"
            markerWidth="12"
            markerHeight="12"
            refX="11"
            refY="6"
            orient="auto-start-reverse"
            markerUnits="userSpaceOnUse"
          >
            <path d="M 2 1 L 11 6 L 2 11" className="lane-marker lane-marker--open" />
          </marker>
        </defs>

        <g transform={`translate(${view.x} ${view.y}) scale(${view.zoom})`}>
          {/* Los marcos son el fondo de lo que encierran. */}
          {placed.frames.map((frame) => (
            <g
              key={frame.id}
              className="lane-frame"
              data-chosen={chosen.has(frame.id) || undefined}
              onClick={pick(frame.id)}
            >
              <rect
                className="lane-frame__box"
                x={frame.rect.x}
                y={frame.rect.y}
                width={frame.rect.width}
                height={frame.rect.height}
              />
              <LaneText
                label={labels[frame.id]}
                x={frame.rect.x + 8}
                y={frame.rect.y + 3}
                anchor="start"
                className="lane-frame__tag"
              />
              {frame.dividers.map((divider) => (
                <g key={divider.id}>
                  <line
                    className="lane-frame__divider"
                    x1={frame.rect.x}
                    x2={frame.rect.x + frame.rect.width}
                    y1={divider.y}
                    y2={divider.y}
                  />
                  <LaneText
                    label={labels[divider.id]}
                    x={frame.rect.x + 8}
                    y={divider.y + 4}
                    anchor="start"
                    className="lane-frame__tag"
                  />
                </g>
              ))}
            </g>
          ))}

          {placed.lanes.map((lane) => (
            <g key={`vida-${lane.node}`}>
              <line
                className="lane-lifeline"
                x1={lane.x}
                x2={lane.x}
                y1={lane.top}
                y2={lane.bottom}
              />
              {lane.destroyed && (
                <path
                  className="lane-cross"
                  d={`M ${lane.x - 8} ${lane.bottom - 8} L ${lane.x + 8} ${lane.bottom + 8} M ${lane.x + 8} ${lane.bottom - 8} L ${lane.x - 8} ${lane.bottom + 8}`}
                />
              )}
            </g>
          ))}

          {placed.bars.map((bar, index) => (
            <rect
              key={`barra-${index}`}
              className="lane-bar"
              x={bar.rect.x}
              y={bar.rect.y}
              width={bar.rect.width}
              height={bar.rect.height}
            />
          ))}

          {placed.lanes.map((lane) => {
            const node = doc.nodes.find((n) => n.id === lane.node);
            const style = node ? nodeStyle(node) : undefined;

            return (
              <g
                key={`cab-${lane.node}`}
                className="lane-header"
                data-chosen={chosen.has(lane.node) || undefined}
                onClick={pick(lane.node)}
              >
                <rect
                  className="lane-header__box"
                  x={lane.header.x}
                  y={lane.header.y}
                  width={lane.header.width}
                  height={lane.header.height}
                  rx={4}
                  style={{ fill: style?.fill, stroke: style?.stroke }}
                />
                <LaneText
                  label={labels[lane.node]}
                  x={lane.header.x + lane.header.width / 2}
                  y={
                    lane.header.y +
                    lane.header.height / 2 -
                    headerHalf(labels[lane.node])
                  }
                  className="lane-header__name"
                  color={style?.textColor}
                />
              </g>
            );
          })}

          {placed.arrows.map((arrow) => {
            const path =
              arrow.loopHeight === undefined
                ? undefined
                : `M ${arrow.fromX} ${arrow.y} L ${arrow.fromX + SELF_LOOP_WIDTH} ${arrow.y} ` +
                  `L ${arrow.fromX + SELF_LOOP_WIDTH} ${arrow.y + arrow.loopHeight} ` +
                  `L ${arrow.toX} ${arrow.y + arrow.loopHeight}`;
            const marker =
              arrow.kind === "sync" || arrow.kind === "destroy"
                ? "url(#lane-arrow-filled)"
                : "url(#lane-arrow-open)";
            const dashed = arrow.kind === "reply" || arrow.kind === "create";

            return (
              <g
                key={arrow.id}
                className="lane-arrow"
                data-chosen={chosen.has(arrow.id) || undefined}
                data-dashed={dashed || undefined}
                onClick={pick(arrow.id)}
              >
                {path ? (
                  <>
                    <path
                      className="lane-arrow__hit"
                      d={path}
                      strokeWidth={HIT_WIDTH}
                    />
                    <path className="lane-arrow__line" d={path} markerEnd={marker} />
                  </>
                ) : (
                  <>
                    <line
                      className="lane-arrow__hit"
                      x1={arrow.fromX}
                      x2={arrow.toX}
                      y1={arrow.y}
                      y2={arrow.y}
                      strokeWidth={HIT_WIDTH}
                    />
                    <line
                      className="lane-arrow__line"
                      x1={arrow.fromX}
                      x2={arrow.toX}
                      y1={arrow.y}
                      y2={arrow.y}
                      markerEnd={marker}
                    />
                  </>
                )}
                <LaneText
                  label={labels[arrow.id]}
                  x={arrow.textAt.x + widthOf(labels[arrow.id]) / 2}
                  y={arrow.textAt.y}
                  className="lane-arrow__text"
                />
              </g>
            );
          })}

          {placed.notes.map((note) => (
            <g
              key={note.id}
              className="lane-note"
              data-chosen={chosen.has(note.id) || undefined}
              onClick={pick(note.id)}
            >
              <path
                className="lane-note__box"
                d={`M ${note.rect.x} ${note.rect.y} L ${note.rect.x + note.rect.width - 9} ${note.rect.y} L ${note.rect.x + note.rect.width} ${note.rect.y + 9} L ${note.rect.x + note.rect.width} ${note.rect.y + note.rect.height} L ${note.rect.x} ${note.rect.y + note.rect.height} Z`}
              />
              <path
                className="lane-note__fold"
                d={`M ${note.rect.x + note.rect.width - 9} ${note.rect.y} L ${note.rect.x + note.rect.width - 9} ${note.rect.y + 9} L ${note.rect.x + note.rect.width} ${note.rect.y + 9}`}
              />
              <LaneText
                label={labels[note.id]}
                x={note.rect.x + note.rect.width / 2}
                y={note.rect.y + note.rect.height / 2 - headerHalf(labels[note.id])}
                className="lane-note__text"
              />
            </g>
          ))}
        </g>
      </svg>

      <div className="lane-canvas__controls">
        <button type="button" onClick={fit} title="Encuadrar el diagrama">
          Encuadrar
        </button>
        <button
          type="button"
          onClick={() => setView({ x: 0, y: 0, zoom: 1 })}
          title="Volver al tamano original"
        >
          100%
        </button>
      </div>

      {doc.nodes.length === 0 && (
        <p className="lane-canvas__empty">
          Un diagrama de secuencia empieza por sus participantes. Anade el primero desde
          la barra de herramientas.
        </p>
      )}
    </div>
  );
}

/** Medio alto de un bloque de texto, para centrarlo verticalmente. */
function headerHalf(label?: { lines: unknown[]; lineHeight: number }): number {
  return label ? (label.lines.length * label.lineHeight) / 2 : 0;
}

/**
 * Ancho de un bloque de texto.
 *
 * El reparto deja `textAt` en la esquina superior izquierda, que es lo
 * inequivoco; para centrarlo en SVG hay que sumarle la mitad del ancho.
 */
function widthOf(label?: { lines: { width: number }[] }): number {
  return label ? label.lines.reduce((acc, line) => Math.max(acc, line.width), 0) : 0;
}
