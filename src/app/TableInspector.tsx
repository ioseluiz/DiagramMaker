import type { ErColumn } from "@/model/generated/ErColumn";
import type { ErTable } from "@/model/generated/ErTable";
import type { Node } from "@/model/generated/Node";
import { nodeStyle } from "@/model/style";
import { useDocumentStore } from "@/store/documentStore";
import { ColorPicker } from "./ColorPicker";
import {
  addColumn,
  moveColumn,
  removeColumn,
  setLabel,
  setNodeFill,
  setTableAlias,
  setTableNote,
  updateColumn,
} from "@/store/operations";

/**
 * Editor de una tabla entidad-relacion.
 *
 * Las marcas son interruptores y no casillas a proposito: ocupan la mitad y
 * se leen igual que en el propio nodo, donde aparecen como `PK`, `U` o `NN`.
 *
 * `FK` no esta: no se marca a mano, se deduce de las relaciones al importar.
 */
export interface TableInspectorProps {
  node: Node;
  table: ErTable;
}

export function TableInspector({ node, table }: TableInspectorProps) {
  const edit = useDocumentStore((s) => s.edit);

  return (
    <>
      <h2 className="inspector__title">Tabla</h2>

      <label className="inspector__field">
        <span className="inspector__label">Nombre</span>
        <input
          value={node.label}
          onChange={(e) =>
            edit(`label:${node.id}`, (d) => setLabel(d, node.id, e.target.value), true)
          }
        />
      </label>

      <label className="inspector__field">
        <span className="inspector__label">Alias</span>
        <input
          value={table.alias ?? ""}
          placeholder="opcional"
          onChange={(e) =>
            edit(
              `alias:${node.id}`,
              (d) => setTableAlias(d, node.id, e.target.value),
              true,
            )
          }
        />
      </label>

      <div className="inspector__field">
        <span className="inspector__label">Color</span>
        <ColorPicker
          value={nodeStyle(node).fill ?? null}
          onChange={(fill) => edit("color", (d) => setNodeFill(d, node.id, fill))}
        />
      </div>

      <div className="members__head">
        <span className="inspector__label">Columnas ({table.columns.length})</span>
        <button
          type="button"
          className="members__add"
          onClick={() => edit("anadir columna", (d) => void addColumn(d, node.id))}
        >
          + Añadir
        </button>
      </div>

      <ul className="columns">
        {table.columns.map((column, index) => (
          <ColumnRow
            key={column.id}
            nodeId={node.id}
            column={column}
            first={index === 0}
            last={index === table.columns.length - 1}
          />
        ))}
      </ul>

      {table.columns.length === 0 && (
        <p className="inspector__hint">
          Una tabla sin columnas se dibuja como una caja con su nombre.
        </p>
      )}

      <label className="inspector__field">
        <span className="inspector__label">Nota</span>
        <textarea
          rows={2}
          value={table.note ?? ""}
          placeholder="opcional"
          onChange={(e) =>
            edit(
              `note:${node.id}`,
              (d) => setTableNote(d, node.id, e.target.value),
              true,
            )
          }
        />
      </label>
    </>
  );
}

interface ColumnRowProps {
  nodeId: string;
  column: ErColumn;
  first: boolean;
  last: boolean;
}

function ColumnRow({ nodeId, column, first, last }: ColumnRowProps) {
  const edit = useDocumentStore((s) => s.edit);

  const change = (changes: Parameters<typeof updateColumn>[3], coalesce = false) =>
    edit(
      `column:${column.id}`,
      (d) => updateColumn(d, nodeId, column.id, changes),
      coalesce,
    );

  return (
    <li className="members__row">
      <div className="members__line">
        <input
          className="members__name"
          value={column.name}
          aria-label="Nombre de la columna"
          onChange={(e) => change({ name: e.target.value }, true)}
        />
        <input
          className="members__type"
          value={column.dataType}
          placeholder="tipo"
          aria-label="Tipo de la columna"
          onChange={(e) => change({ dataType: e.target.value }, true)}
        />
      </div>

      <div className="members__line">
        <Flag
          label="PK"
          title="Clave primaria"
          active={column.primaryKey}
          onToggle={() => change({ primaryKey: !column.primaryKey })}
        />
        <Flag
          label="U"
          title="Valor unico"
          active={column.unique}
          onToggle={() => change({ unique: !column.unique })}
        />
        <Flag
          label="NN"
          title="No admite nulos"
          active={!column.nullable}
          disabled={column.primaryKey}
          onToggle={() => change({ nullable: !column.nullable })}
        />
        <Flag
          label="AI"
          title="Autoincremental"
          active={column.increment}
          onToggle={() => change({ increment: !column.increment })}
        />
        {column.foreignKey && (
          <span className="members__derived" title="Deducido de las relaciones">
            FK
          </span>
        )}

        <span className="inspector__spacer" />

        <button
          type="button"
          className="members__move"
          title="Subir"
          aria-label={`Subir ${column.name}`}
          disabled={first}
          onClick={() =>
            edit("mover columna", (d) => moveColumn(d, nodeId, column.id, -1))
          }
        >
          ▲
        </button>
        <button
          type="button"
          className="members__move"
          title="Bajar"
          aria-label={`Bajar ${column.name}`}
          disabled={last}
          onClick={() =>
            edit("mover columna", (d) => moveColumn(d, nodeId, column.id, 1))
          }
        >
          ▼
        </button>
        <button
          type="button"
          className="members__delete"
          title="Borrar columna"
          aria-label={`Borrar ${column.name}`}
          onClick={() =>
            edit("borrar columna", (d) => removeColumn(d, nodeId, column.id))
          }
        >
          ×
        </button>
      </div>
    </li>
  );
}

interface FlagProps {
  label: string;
  title: string;
  active: boolean;
  disabled?: boolean;
  onToggle: () => void;
}

function Flag({ label, title, active, disabled, onToggle }: FlagProps) {
  return (
    <button
      type="button"
      className="members__flag"
      title={disabled ? `${title} (implicito en una clave primaria)` : title}
      aria-pressed={active}
      disabled={disabled}
      onClick={onToggle}
    >
      {label}
    </button>
  );
}
