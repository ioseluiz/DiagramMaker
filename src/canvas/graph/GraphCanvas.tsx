import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  Controls,
  MiniMap,
  ReactFlow,
  ReactFlowProvider,
  type Connection,
  type Edge as FlowEdge,
  type EdgeChange,
  type NodeChange,
  type OnSelectionChangeParams,
} from "@xyflow/react";
import { useCallback, useMemo } from "react";
import "@xyflow/react/dist/style.css";

import type { Document } from "@/model/generated/Document";
import { useDocumentStore } from "@/store/documentStore";
import type { Side } from "@/model/generated/Side";
import {
  connect,
  moveNode,
  reconnect,
  removeElements,
  resizeNode,
} from "@/store/operations";
import { EdgeMarkers } from "./EdgeMarkers";
import { ShapeNode } from "./ShapeNode";
import { toAbsolute, toFlowEdges, toFlowNodes, type DiagramFlowNode } from "./adapter";
import "./GraphCanvas.css";

const NODE_TYPES = { shape: ShapeNode };

const SNAP: [number, number] = [8, 8];

export interface GraphCanvasProps {
  document: Document;
}

function GraphCanvasImpl({ document }: GraphCanvasProps) {
  const edit = useDocumentStore((s) => s.edit);
  const select = useDocumentStore((s) => s.select);
  const selection = useDocumentStore((s) => s.selection);

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
      edit("conectar", (draft) => {
        connect(draft, connection.source, connection.target, "association", {
          source: (connection.sourceHandle as Side | null) ?? undefined,
          target: (connection.targetHandle as Side | null) ?? undefined,
        });
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
      const handle = movedTarget ? connection.targetHandle : connection.sourceHandle;

      edit("reenganchar", (draft) => {
        reconnect(draft, previous.id, end, node, (handle as Side | null) ?? undefined);
      });
    },
    [edit],
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
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onConnect={onConnect}
        onReconnect={onReconnect}
        onSelectionChange={onSelectionChange}
        connectionMode={ConnectionMode.Loose}
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
