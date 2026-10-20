import type { ClassBox } from "@/model/generated/ClassBox";
import type { ClassMember } from "@/model/generated/ClassMember";
import type { Node } from "@/model/generated/Node";
import type { Visibility } from "@/model/generated/Visibility";
import { SIGIL, VISIBILITY_LABEL } from "@/model/classBox";
import { nodeStyle } from "@/model/style";
import { useDocumentStore } from "@/store/documentStore";
import { ColorPicker } from "./ColorPicker";
import {
  addClassMember,
  moveClassMember,
  removeClassMember,
  setClassStereotype,
  setLabel,
  setNodeFill,
  updateClassMember,
} from "@/store/operations";

/**
 * Editor de una caja de clase.
 *
 * Los dos compartimentos se editan igual porque son lo mismo —una lista
 * ordenada de miembros— y solo se diferencian en que una operacion tiene
 * parametros. De ahi que la fila sea una sola y reciba `isOperation`.
 *
 * Lo estatico y lo abstracto son interruptores por el mismo motivo que las
 * marcas de una columna: ocupan la mitad que una casilla y se leen con el
 * mismo signo que el nodo dibuja.
 */
export interface ClassInspectorProps {
  node: Node;
  box: ClassBox;
}

/** Estereotipos de uso corriente, para no tener que escribirlos. */
const COMMON_STEREOTYPES = ["interface", "abstract", "enumeration", "record"];

export function ClassInspector({ node, box }: ClassInspectorProps) {
  const edit = useDocumentStore((s) => s.edit);

  return (
    <>
      <h2 className="inspector__title">Clase</h2>

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
        <span className="inspector__label">Estereotipo</span>
        <input
          value={box.stereotype ?? ""}
          placeholder="opcional"
          list="class-stereotypes"
          onChange={(e) =>
            edit(
              `stereotype:${node.id}`,
              (d) => setClassStereotype(d, node.id, e.target.value),
              true,
            )
          }
        />
      </label>
      <datalist id="class-stereotypes">
        {COMMON_STEREOTYPES.map((name) => (
          <option key={name} value={name} />
        ))}
      </datalist>

      <div className="inspector__field">
        <span className="inspector__label">Color</span>
        <ColorPicker
          value={nodeStyle(node).fill ?? null}
          onChange={(fill) => edit("color", (d) => setNodeFill(d, node.id, fill))}
        />
      </div>

      <Compartment
        nodeId={node.id}
        title="Atributos"
        compartment="attributes"
        members={box.attributes}
        isOperation={false}
      />

      <Compartment
        nodeId={node.id}
        title="Operaciones"
        compartment="operations"
        members={box.operations}
        isOperation
      />
    </>
  );
}

interface CompartmentProps {
  nodeId: string;
  title: string;
  compartment: "attributes" | "operations";
  members: ClassMember[];
  isOperation: boolean;
}

function Compartment({
  nodeId,
  title,
  compartment,
  members,
  isOperation,
}: CompartmentProps) {
  const edit = useDocumentStore((s) => s.edit);

  return (
    <>
      <div className="members__head">
        <span className="inspector__label">
          {title} ({members.length})
        </span>
        <button
          type="button"
          className="members__add"
          onClick={() =>
            edit(`anadir ${title}`, (d) => void addClassMember(d, nodeId, compartment))
          }
        >
          + Añadir
        </button>
      </div>

      <ul className="members">
        {members.map((member, index) => (
          <MemberRow
            key={member.id}
            nodeId={nodeId}
            member={member}
            isOperation={isOperation}
            first={index === 0}
            last={index === members.length - 1}
          />
        ))}
      </ul>
    </>
  );
}

interface MemberRowProps {
  nodeId: string;
  member: ClassMember;
  isOperation: boolean;
  first: boolean;
  last: boolean;
}

function MemberRow({ nodeId, member, isOperation, first, last }: MemberRowProps) {
  const edit = useDocumentStore((s) => s.edit);

  const change = (changes: Partial<ClassMember>, coalesce = false) =>
    edit(
      `member:${member.id}`,
      (d) => updateClassMember(d, nodeId, member.id, changes),
      coalesce,
    );

  return (
    <li className="members__row">
      <div className="members__line">
        <select
          className="members__sigil"
          value={member.visibility}
          aria-label={`Visibilidad de ${member.name}`}
          title={VISIBILITY_LABEL[member.visibility]}
          onChange={(e) => change({ visibility: e.target.value as Visibility })}
        >
          {(Object.keys(SIGIL) as Visibility[]).map((visibility) => (
            <option key={visibility} value={visibility}>
              {SIGIL[visibility]}
            </option>
          ))}
        </select>

        <input
          className="members__name"
          value={member.name}
          aria-label="Nombre del miembro"
          onChange={(e) => change({ name: e.target.value }, true)}
        />
        <input
          className="members__type"
          value={member.typeName}
          placeholder={isOperation ? "devuelve" : "tipo"}
          aria-label={isOperation ? "Tipo que devuelve" : "Tipo del atributo"}
          onChange={(e) => change({ typeName: e.target.value }, true)}
        />
      </div>

      {isOperation && (
        <div className="members__line">
          <input
            className="members__params"
            value={member.parameters}
            placeholder="parametros, tal como se escriben"
            aria-label={`Parametros de ${member.name}`}
            onChange={(e) => change({ parameters: e.target.value }, true)}
          />
        </div>
      )}

      <div className="members__line">
        <Flag
          label="S"
          title="Estatico: se dibuja subrayado"
          active={member.isStatic ?? false}
          onToggle={() => change({ isStatic: !member.isStatic })}
        />
        <Flag
          label="A"
          title="Abstracto: se dibuja en cursiva"
          active={member.isAbstract ?? false}
          onToggle={() => change({ isAbstract: !member.isAbstract })}
        />

        <span className="inspector__spacer" />

        <button
          type="button"
          className="members__move"
          title="Subir"
          aria-label={`Subir ${member.name}`}
          disabled={first}
          onClick={() =>
            edit("mover miembro", (d) => moveClassMember(d, nodeId, member.id, -1))
          }
        >
          ▲
        </button>
        <button
          type="button"
          className="members__move"
          title="Bajar"
          aria-label={`Bajar ${member.name}`}
          disabled={last}
          onClick={() =>
            edit("mover miembro", (d) => moveClassMember(d, nodeId, member.id, 1))
          }
        >
          ▼
        </button>
        <button
          type="button"
          className="members__delete"
          title="Borrar miembro"
          aria-label={`Borrar ${member.name}`}
          onClick={() =>
            edit("borrar miembro", (d) => removeClassMember(d, nodeId, member.id))
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
  onToggle: () => void;
}

function Flag({ label, title, active, onToggle }: FlagProps) {
  return (
    <button
      type="button"
      className="members__flag"
      title={title}
      aria-pressed={active}
      onClick={onToggle}
    >
      {label}
    </button>
  );
}
