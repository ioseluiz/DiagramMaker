import { useCallback } from "react";

import type { DiagramKind } from "@/model/generated/DiagramKind";
import type { NodeShape } from "@/model/generated/NodeShape";
import { SHAPE_LABEL } from "@/model/shapes";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { addClass, addNode, addTable } from "@/store/operations";

/**
 * Formas que ofrece la paleta, segun el tipo de diagrama.
 *
 * En un entidad-relacion lo que se dibuja son tablas; ofrecer un rombo o un
 * actor seria ruido. El resto de tipos comparten el juego general hasta que
 * cada uno tenga el suyo.
 */
const PALETTE_BY_KIND: Partial<Record<DiagramKind, NodeShape[]>> = {
  er: ["table", "package", "note", "text"],
  class: ["class", "package", "note", "text"],
  // En una maquina de estados lo que se dibuja son estados redondeados, sus
  // extremos y las bifurcaciones; un cilindro o un actor ahi serian ruido.
  state: [
    "roundedRect",
    "initialNode",
    "finalNode",
    "diamond",
    "bar",
    "package",
    "note",
    "text",
  ],
  activity: [
    "roundedRect",
    "initialNode",
    "finalNode",
    "diamond",
    "bar",
    "package",
    "note",
    "text",
  ],
  // Un caso de uso es una elipse dentro de la frontera del sistema, y quien
  // lo usa, un actor.
  useCase: ["ellipse", "actor", "package", "rectangle", "note", "text"],
  // Una secuencia no se dibuja colocando formas: sus participantes y sus
  // pasos se anaden desde el panel del guion, que es donde esta el orden.
  sequence: [],
};

const DEFAULT_PALETTE: NodeShape[] = [
  "rectangle",
  "roundedRect",
  "ellipse",
  "diamond",
  "hexagon",
  "cylinder",
  "actor",
  "package",
  "note",
  "text",
];

/** Donde cae la siguiente forma: una rejilla de cuatro en fondo. */
function nextPosition(count: number): { x: number; y: number } {
  const column = count % 4;
  const row = Math.floor(count / 4);
  return { x: 80 + column * 200, y: 80 + row * 120 };
}

/**
 * Anadir una forma al diagrama.
 *
 * Lo usan la barra de herramientas y la paleta de comandos, que son dos
 * maneras de pedir lo mismo. Vivia dentro de la barra; salio de ahi para no
 * tener dos copias que pudieran separarse.
 */
export function useAddShape(): {
  palette: NodeShape[];
  addShape: (shape: NodeShape) => void;
} {
  const doc = useDocumentStore(selectDocument);
  const edit = useDocumentStore((s) => s.edit);

  const addShape = useCallback(
    (shape: NodeShape) => {
      const position = nextPosition(
        useDocumentStore.getState().history?.present.nodes.length ?? 0,
      );

      edit("anadir", (draft) => {
        if (shape === "table") {
          addTable(draft, position);
          return;
        }
        if (shape === "class") {
          addClass(draft, position);
          return;
        }
        addNode(draft, { shape, label: SHAPE_LABEL[shape], ...position });
      });
    },
    [edit],
  );

  return {
    palette: (doc && PALETTE_BY_KIND[doc.kind]) ?? DEFAULT_PALETTE,
    addShape,
  };
}
