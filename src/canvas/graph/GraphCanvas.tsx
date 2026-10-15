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
import { useCanvasTool } from "@/app/useCanvasTool";
import { useDocumentStore } from "@/store/documentStore";
import {
  RELATION_LABELS,
  addColumn,
  connect,
  duplicateNode,
  moveNode,
  reconnect,
  relationKindOf,
  removeElements,
  resizeNode,
  reverseEdge,
  setNodeParent,
  setRelationKind,
  groupNodes,
  ungroupNode,
  type RelationKind,
} from "@/store/operations";
import { erTableOf } from "@/model/table";
import { isContainer } from "@/model/shapes";
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

const CARDINALITY_LABEL: Record<RelationKind, string> = {
  manyToOne: "Muchos a uno",
  oneToMany: "Uno a muchos",
  oneToOne: "Uno a uno",
  manyToMany: "Muchos a muchos",
};

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
  const tool = useCanvasTool((s) => s.tool);
  const hand = tool === "hand";

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

  /**
   * Al soltar un nodo, mira si cayo sobre un contenedor y lo mete dentro.
   *
   * Se usa el centro del nodo y no su caja entera: pedir que encaje por
   * completo obliga a punterias imposibles con tablas grandes, y asomar una
   * esquina fuera del grupo es normal mientras se coloca.
   */
  const onNodeDragStop = useCallback(
    (_event: MouseEvent | TouchEvent, flowNode: { id: string }) => {
      const node = document.nodes.find((n) => n.id === flowNode.id);
      if (!node?.geometry || isContainer(node.shape)) return;

      const center = {
        x: node.geometry.x + node.geometry.width / 2,
        y: node.geometry.y + node.geometry.height / 2,
      };

      const container = document.nodes.find((candidate) => {
        if (candidate.id === node.id || !isContainer(candidate.shape)) return false;
        const g = candidate.geometry;
        if (!g) return false;
        return (
          center.x >= g.x &&
          center.x <= g.x + g.width &&
          center.y >= g.y &&
          center.y <= g.y + g.height
        );
      });

      const next = container?.id ?? null;
      if ((node.parent ?? null) === next) return;

      edit(next ? "agrupar" : "sacar del grupo", (d) =>
        setNodeParent(d, node.id, next),
      );
    },
    [document, edit],
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

      if (isContainer(node.shape)) {
        items.push({
          label: "Desagrupar",
          onSelect: () => edit("desagrupar", (d) => ungroupNode(d, node.id)),
        });
      } else if (node.parent) {
        items.push({
          label: "Sacar del grupo",
          onSelect: () =>
            edit("sacar del grupo", (d) => setNodeParent(d, node.id, null)),
        });
      }

      // Agrupar actua sobre la seleccion, no sobre el nodo pulsado: pedirlo
      // con un solo nodo seleccionado casi siempre es un descuido.
      const groupable = selection.filter((id) => {
        const candidate = document.nodes.find((n) => n.id === id);
        return candidate && !candidate.parent && !isContainer(candidate.shape);
      });

      if (groupable.length >= 2) {
        items.push({
          label: `Agrupar ${groupable.length} elementos`,
          separated: true,
          onSelect: () => edit("agrupar", (d) => void groupNodes(d, groupable)),
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
    [document, edit, selection],
  );

  const onEdgeContextMenu = useCallback(
    (event: React.MouseEvent, flowEdge: { id: string }) => {
      event.preventDefault();

      const relation = document.edges.find((e) => e.id === flowEdge.id);
      if (!relation) return;

      const items: MenuItem[] = [];

      // En un entidad-relacion, cambiar la cardinalidad es lo que mas se hace
      // sobre una relacion ya creada; tenerlo aqui ahorra ir al inspector.
      if (document.kind === "er") {
        const current = relationKindOf(relation);

        for (const kind of Object.keys(RELATION_LABELS) as RelationKind[]) {
          const [from, to] = RELATION_LABELS[kind];
          items.push({
            label: `${CARDINALITY_LABEL[kind]}  (${from} → ${to})`,
            checked: kind === current,
            onSelect: () =>
              edit("cardinalidad", (d) => setRelationKind(d, relation.id, kind)),
          });
        }
      }

      items.push({
        label: "Invertir sentido",
        separated: items.length > 0,
        onSelect: () => edit("invertir", (d) => reverseEdge(d, relation.id)),
      });

      items.push({
        label: "Borrar",
        separated: true,
        danger: true,
        onSelect: () => edit("borrar", (d) => removeElements(d, [relation.id])),
      });

      setMenu({ x: event.clientX, y: event.clientY, items });
    },
    [document, edit],
  );

  const onPaneContextMenu = useCallback(
    (event: React.MouseEvent | MouseEvent) => {
      event.preventDefault();

      const everything = [
        ...document.nodes.map((n) => n.id),
        ...document.edges.map((e) => e.id),
      ];

      const groupable = selection.filter((id) => {
        const node = document.nodes.find((n) => n.id === id);
        return node && !node.parent && !isContainer(node.shape);
      });

      const items: MenuItem[] = [
        {
          label: "Seleccionar todo",
          disabled: everything.length === 0,
          onSelect: () => select(everything),
        },
        {
          label: "Ajustar a la vista",
          onSelect: () => void fitView({ padding: 0.2, duration: 200 }),
        },
      ];

      if (groupable.length >= 2) {
        items.push({
          label: `Agrupar ${groupable.length} elementos`,
          separated: true,
          onSelect: () => edit("agrupar", (d) => void groupNodes(d, groupable)),
        });
      }

      setMenu({ x: event.clientX, y: event.clientY, items });
    },
    [document, edit, fitView, select, selection],
  );

  const onSelectionChange = useCallback(
    ({ nodes: n, edges: e }: OnSelectionChangeParams) => {
      select([...n.map((x) => x.id), ...e.map((x) => x.id)]);
    },
    [select],
  );

  return (
    <div className="graph-canvas" data-tool={tool}>
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
        onNodeDragStop={onNodeDragStop}
        onNodeContextMenu={onNodeContextMenu}
        onEdgeContextMenu={onEdgeContextMenu}
        onPaneContextMenu={onPaneContextMenu}
        connectionMode={ConnectionMode.Loose}
        /*
         * Conectar es arrastrar. Con el valor por defecto, un clic suelto
         * sobre un conector dejaba el lienzo en modo "conectando" hasta el
         * siguiente clic, y eso se siente como que la aplicacion se colgo.
         */
        connectOnClick={false}
        /*
         * Con el puntero, arrastrar sobre el lienzo vacio selecciona por
         * rectangulo; con la mano, desplaza desde cualquier sitio, tambien
         * encima de un nodo. La barra espaciadora hace de mano temporal y el
         * boton central desplaza siempre.
         */
        selectionOnDrag={!hand}
        panOnDrag={hand ? true : MIDDLE_BUTTON}
        nodesDraggable={!hand}
        elementsSelectable={!hand}
        panActivationKeyCode="Space"
        /*
         * La rueda desplaza y Ctrl+rueda hace zoom, como en Figma: con un
         * panel tactil, dos dedos mueven el lienzo. Shift+rueda desplaza en
         * horizontal.
         */
        panOnScroll
        zoomOnScroll={false}
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
