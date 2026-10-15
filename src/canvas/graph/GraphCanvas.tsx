import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  useReactFlow,
  type Connection,
  type Edge as FlowEdge,
  type EdgeChange,
  type NodeChange,
  type OnSelectionChangeParams,
} from "@xyflow/react";
import { useCallback, useMemo, useState } from "react";
import "@xyflow/react/dist/style.css";

import type { Document } from "@/model/generated/Document";
import { useDocumentStore } from "@/store/documentStore";
import {
  addColumn,
  connect,
  duplicateNode,
  moveNode,
  reconnect,
  removeElements,
  resizeNode,
  reverseEdge,
} from "@/store/operations";
import { erTableOf } from "@/model/table";
import { ContextMenu, type MenuItem } from "./ContextMenu";
import { EdgeMarkers } from "./EdgeMarkers";
import { decodeHandle } from "./handles";
import { RelationEdge } from "./RelationEdge";
import { ShapeNode } from "./ShapeNode";
import { TableNode } from "./TableNode";
import { toAbsolute, toFlowEdges, toFlowNodes, type DiagramFlowNode } from "./adapter";
import "./GraphCanvas.css";

const NODE_TYPES = { shape: ShapeNode, table: TableNode };
const EDGE_TYPES = { relation: RelationEdge };

const SNAP: [number, number] = [8, 8];

/** Boton central del raton, el unico que desplaza el lienzo al arrastrar. */
const MIDDLE_BUTTON = [1];

export interface GraphCanvasProps {
  document: Document;
}

/** Que se pulso con el boton derecho y donde. */
interface MenuState {
  x: number;
  y: number;
  items: MenuItem[];
}

function GraphCanvasImpl({ document }: GraphCanvasProps) {
  const edit = useDocumentStore((s) => s.edit);
  const select = useDocumentStore((s) => s.select);
  const selection = useDocumentStore((s) => s.selection);
  const { fitView } = useReactFlow();

  const [menu, setMenu] = useState<MenuState | null>(null);

  const selected = useMemo(() => new Set(selection), [selection]);
  const nodes = useMemo(() => toFlowNodes(document, selected), [document, selected]);
  const edges = useMemo(() => toFlowEdges(document, selected), [document, selected]);

  const onNodesChange = useCallback(
    (changes: NodeChange<DiagramFlowNode>[]) => {
      for (const change of changes) {
        switch (change.type) {
          case "position": {
            if (!change.position) break;
            const { x, y } = toAbsolute(document, change.id, change.position);
            // Mientras `dragging` es true llegan decenas de eventos por
            // segundo; se fusionan en un unico paso de deshacer.
            edit(
              `move:${change.id}`,
              (draft) => moveNode(draft, change.id, x, y),
              change.dragging === true,
            );
            break;
          }

          case "dimensions": {
            if (!change.dimensions) break;
            const { width, height } = change.dimensions;
            edit(
              `resize:${change.id}`,
              (draft) => resizeNode(draft, change.id, width, height),
              change.resizing === true,
            );
            break;
          }

          case "remove":
            edit("borrar", (draft) => removeElements(draft, [change.id]));
            break;
        }
      }
    },
    [document, edit],
  );

  const onEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      const removed = changes.filter((c) => c.type === "remove").map((c) => c.id);
      if (removed.length > 0) {
        edit("borrar", (draft) => removeElements(draft, removed));
      }
    },
    [edit],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      if (!connection.source || !connection.target) return;

      // Un conector de tabla trae ademas la columna; es lo que convierte la
      // union de dos cajas en una clave ajena concreta.
      const from = decodeHandle(connection.sourceHandle);
      const to = decodeHandle(connection.targetHandle);

      edit("conectar", (draft) => {
        connect(
          draft,
          connection.source,
          connection.target,
          "association",
          { source: from?.side, target: to?.side },
          { source: from?.column, target: to?.column },
        );
      });
    },
    [edit],
  );

  /**
   * Arrastrar la punta de una arista la reengancha a otro nodo o a otro lado.
   * Es la forma de corregir un lado sin tener que borrar y rehacer la
   * relacion, que perderia su etiqueta y su tipo.
   */
  const onReconnect = useCallback(
    (previous: FlowEdge, connection: Connection) => {
      if (!connection.source || !connection.target) return;

      const movedTarget = previous.source === connection.source;
      const end = movedTarget ? "target" : "source";
      const node = movedTarget ? connection.target : connection.source;
      const handle = decodeHandle(
        movedTarget ? connection.targetHandle : connection.sourceHandle,
      );

      edit("reenganchar", (draft) => {
        reconnect(draft, previous.id, end, node, handle?.side, handle?.column);
      });
    },
    [edit],
  );

  /* --- Menu contextual ---------------------------------------------- */

  const onNodeContextMenu = useCallback(
    (event: React.MouseEvent, flowNode: { id: string }) => {
      event.preventDefault();

      const node = document.nodes.find((n) => n.id === flowNode.id);
      if (!node) return;

      const items: MenuItem[] = [
        {
          label: "Duplicar",
          onSelect: () => edit("duplicar", (d) => void duplicateNode(d, node.id)),
        },
      ];

      if (erTableOf(node)) {
        items.push({
          label: "Añadir columna",
          onSelect: () => edit("anadir columna", (d) => void addColumn(d, node.id)),
        });
      }

      items.push({
        label: "Borrar",
        separated: true,
        danger: true,
        onSelect: () => edit("borrar", (d) => removeElements(d, [node.id])),
      });

      setMenu({ x: event.clientX, y: event.clientY, items });
    },
    [document, edit],
  );

  const onEdgeContextMenu = useCallback(
    (event: React.MouseEvent, flowEdge: { id: string }) => {
      event.preventDefault();

      setMenu({
        x: event.clientX,
        y: event.clientY,
        items: [
          {
            label: "Invertir sentido",
            onSelect: () => edit("invertir", (d) => reverseEdge(d, flowEdge.id)),
          },
          {
            label: "Borrar",
            separated: true,
            danger: true,
            onSelect: () => edit("borrar", (d) => removeElements(d, [flowEdge.id])),
          },
        ],
      });
    },
    [edit],
  );

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();

      const everything = [
        ...document.nodes.map((n) => n.id),
        ...document.edges.map((e) => e.id),
      ];

      setMenu({
        x: event.clientX,
        y: event.clientY,
        items: [
          {
            label: "Seleccionar todo",
            disabled: everything.length === 0,
            onSelect: () => select(everything),
          },
          {
            label: "Ajustar a la vista",
            onSelect: () => void fitView({ padding: 0.2, duration: 200 }),
          },
        ],
      });
    },
    [document, fitView, select],
  );

  const onSelectionChange = useCallback(
    ({ nodes: n, edges: e }: OnSelectionChangeParams) => {
      select([...n.map((x) => x.id), ...e.map((x) => x.id)]);
    },
    [select],
  );

  return (
    <div className="graph-canvas">
      <EdgeMarkers />
      <ReactFlow
        nodes={nodes}
        edges={edges}
        nodeTypes={NODE_TYPES}
        edgeTypes={EDGE_TYPES}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onReconnect={onReconnect}
        onSelectionChange={onSelectionChange}
        onNodeContextMenu={onNodeContextMenu}
        onEdgeContextMenu={onEdgeContextMenu}
        onPaneContextMenu={onPaneContextMenu}
        connectionMode={ConnectionMode.Loose}
        /*
         * Arrastrar sobre el lienzo vacio selecciona por rectangulo, que es
         * lo que se espera en una herramienta de dibujo. Para desplazar el
         * lienzo quedan la rueda del raton, el boton central y la barra
         * espaciadora, que es el reparto de draw.io y Figma.
         */
        selectionOnDrag
        panOnDrag={MIDDLE_BUTTON}
        panActivationKeyCode="Space"
        selectionKeyCode="Shift"
        multiSelectionKeyCode={["Control", "Meta"]}
        snapToGrid
        snapGrid={SNAP}
        minZoom={0.1}
        maxZoom={4}
        fitView
        fitViewOptions={{ padding: 0.2, maxZoom: 1 }}
        proOptions={{ hideAttribution: false }}
        deleteKeyCode={["Delete", "Backspace"]}
      >
        <Background variant={BackgroundVariant.Dots} gap={16} size={1} />
        <Controls showInteractive={false} />
        <MiniMap pannable zoomable />
      </ReactFlow>

      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          items={menu.items}
          onClose={() => setMenu(null)}
        />
      )}
    </div>
  );
}

export function GraphCanvas(props: GraphCanvasProps) {
  return (
    <ReactFlowProvider>
      <GraphCanvasImpl {...props} />
    </ReactFlowProvider>
  );
}
