import type { Document } from "@/model/generated/Document";
import type { BlockKind } from "@/model/generated/BlockKind";
import type { MessageKind } from "@/model/generated/MessageKind";
import type { NotePlacement } from "@/model/generated/NotePlacement";
import type { Step } from "@/model/generated/Step";
import {
  BLOCK_KIND_LABEL,
  MESSAGE_KIND_LABEL,
  NOTE_PLACEMENT_LABEL,
  describeStep,
  flatten,
  locate,
  scriptOf,
} from "@/model/sequence";
import { useDocumentStore } from "@/store/documentStore";
import {
  addBranch,
  addMessage,
  addParticipant,
  addSequenceBlock,
  addSequenceNote,
  moveParticipant,
  moveStep,
  removeElements,
  removeStep,
  setBranchLabel,
  setLabel,
  setMessageActivation,
  setMessageEndpoint,
  setMessageKind,
  setNotePlacement,
  setStepText,
} from "@/store/operations";
import "./SequenceInspector.css";

/**
 * Edicion de un diagrama de secuencia.
 *
 * La lista ordenada no es una vista alternativa del lienzo: es *la* forma de
 * editar. En una secuencia no hay coordenadas que arrastrar —el reparto las
 * decide— asi que mover un mensaje es cambiarlo de sitio en la lista. El
 * lienzo y la lista comparten seleccion, de modo que pinchar en uno resalta
 * en el otro.
 */
export interface SequenceInspectorProps {
  document: Document;
}

export function SequenceInspector({ document: doc }: SequenceInspectorProps) {
  const selection = useDocumentStore((s) => s.selection);
  const select = useDocumentStore((s) => s.select);
  const edit = useDocumentStore((s) => s.edit);

  const only = selection.length === 1 ? selection[0] : undefined;
  const participant = only ? doc.nodes.find((n) => n.id === only) : undefined;
  const located = only ? locate(scriptOf(doc).steps, only) : undefined;

  const nameOf = (id: string) => doc.nodes.find((n) => n.id === id)?.label ?? "?";
  const steps = flatten(scriptOf(doc).steps);

  /** Detras de que paso insertar lo nuevo: el elegido, o el final. */
  const after = located?.step.id;

  function newParticipant() {
    edit("participante", (d) => {
      const id = addParticipant(d, `Participante ${d.nodes.length + 1}`);
      select([id]);
    });
  }

  function newMessage() {
    const [first, second] = doc.nodes;
    if (!first) return;

    edit("mensaje", (d) => {
      const id = addMessage(d, {
        from: first.id,
        to: (second ?? first).id,
        after,
      });
      select([id]);
    });
  }

  return (
    <>
      {participant && (
        <>
          <h2 className="inspector__title">Participante</h2>
          <Row label="Nombre">
            <input
              value={participant.label}
              onChange={(e) =>
                edit(
                  `label:${participant.id}`,
                  (d) => setLabel(d, participant.id, e.target.value),
                  true,
                )
              }
            />
          </Row>
          <Row label="Orden">
            <div className="seq__buttons">
              <button
                type="button"
                onClick={() =>
                  edit("orden", (d) => moveParticipant(d, participant.id, -1))
                }
                title="Mover a la izquierda"
              >
                ←
              </button>
              <button
                type="button"
                onClick={() =>
                  edit("orden", (d) => moveParticipant(d, participant.id, 1))
                }
                title="Mover a la derecha"
              >
                →
              </button>
              <button
                type="button"
                onClick={() =>
                  edit("borrar", (d) => removeElements(d, [participant.id]))
                }
                title="Quitar el participante y lo que dependa de el"
              >
                Quitar
              </button>
            </div>
          </Row>
        </>
      )}

      {located && <StepEditor document={doc} step={located.step} nameOf={nameOf} />}

      <h2 className="inspector__title">Guion</h2>

      <div className="seq__buttons seq__buttons--wrap">
        <button type="button" onClick={newParticipant}>
          + Participante
        </button>
        <button type="button" onClick={newMessage} disabled={doc.nodes.length === 0}>
          + Mensaje
        </button>
        <button
          type="button"
          disabled={doc.nodes.length === 0}
          onClick={() =>
            edit("marco", (d) => select([addSequenceBlock(d, "alt", "", after)]))
          }
        >
          + Marco
        </button>
        <button
          type="button"
          disabled={doc.nodes.length === 0}
          onClick={() =>
            edit("nota", (d) => {
              const first = d.nodes[0];
              if (first) {
                select([addSequenceNote(d, [first.id], "over", "Nota", after)]);
              }
            })
          }
        >
          + Nota
        </button>
      </div>

      {steps.length === 0 ? (
        <p className="inspector__hint">
          El guion esta vacio. Un mensaje necesita al menos un participante.
        </p>
      ) : (
        <ol className="seq__list">
          {steps.map(({ step, depth }) => (
            <li
              key={step.id}
              className="seq__step"
              data-chosen={selection.includes(step.id) || undefined}
              style={{ paddingLeft: 8 + depth * 14 }}
            >
              <button
                type="button"
                className="seq__step-label"
                onClick={() => select([step.id])}
                title={describeStep(step, nameOf)}
              >
                {describeStep(step, nameOf)}
              </button>
              <span className="seq__step-actions">
                <button
                  type="button"
                  onClick={() => edit("orden", (d) => moveStep(d, step.id, -1))}
                  title="Subir"
                >
                  ↑
                </button>
                <button
                  type="button"
                  onClick={() => edit("orden", (d) => moveStep(d, step.id, 1))}
                  title="Bajar"
                >
                  ↓
                </button>
                <button
                  type="button"
                  onClick={() => edit("borrar", (d) => removeStep(d, step.id))}
                  title="Quitar"
                >
                  ×
                </button>
              </span>
            </li>
          ))}
        </ol>
      )}
    </>
  );
}

/* --- Propiedades del paso elegido ---------------------------------------- */

function StepEditor({
  document: doc,
  step,
  nameOf,
}: {
  document: Document;
  step: Step;
  nameOf: (id: string) => string;
}) {
  const edit = useDocumentStore((s) => s.edit);

  if (step.type === "message") {
    return (
      <>
        <h2 className="inspector__title">Mensaje</h2>
        <Row label="De">
          <select
            value={step.from}
            onChange={(e) =>
              edit("origen", (d) =>
                setMessageEndpoint(d, step.id, "from", e.target.value),
              )
            }
          >
            {doc.nodes.map((node) => (
              <option key={node.id} value={node.id}>
                {node.label}
              </option>
            ))}
          </select>
        </Row>
        <Row label="A">
          <select
            value={step.to}
            onChange={(e) =>
              edit("destino", (d) =>
                setMessageEndpoint(d, step.id, "to", e.target.value),
              )
            }
          >
            {doc.nodes.map((node) => (
              <option key={node.id} value={node.id}>
                {node.label}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Tipo">
          <select
            value={step.kind}
            onChange={(e) =>
              edit("tipo", (d) =>
                setMessageKind(d, step.id, e.target.value as MessageKind),
              )
            }
          >
            {Object.entries(MESSAGE_KIND_LABEL).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Texto">
          <input
            value={step.text ?? ""}
            onChange={(e) =>
              edit(
                `texto:${step.id}`,
                (d) => setStepText(d, step.id, e.target.value),
                true,
              )
            }
          />
        </Row>
        <Row label="Ejecucion">
          <div className="seq__checks">
            <label>
              <input
                type="checkbox"
                checked={step.activate ?? false}
                onChange={(e) =>
                  edit("activar", (d) =>
                    setMessageActivation(d, step.id, "activate", e.target.checked),
                  )
                }
              />
              Abre en {nameOf(step.to)}
            </label>
            <label>
              <input
                type="checkbox"
                checked={step.deactivate ?? false}
                onChange={(e) =>
                  edit("desactivar", (d) =>
                    setMessageActivation(d, step.id, "deactivate", e.target.checked),
                  )
                }
              />
              Cierra en {nameOf(step.from)}
            </label>
          </div>
        </Row>
      </>
    );
  }

  if (step.type === "note") {
    return (
      <>
        <h2 className="inspector__title">Nota</h2>
        <Row label="Donde">
          <select
            value={step.placement}
            onChange={(e) =>
              edit("colocacion", (d) =>
                setNotePlacement(d, step.id, e.target.value as NotePlacement),
              )
            }
          >
            {Object.entries(NOTE_PLACEMENT_LABEL).map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </Row>
        <Row label="Texto">
          <input
            value={step.text}
            onChange={(e) =>
              edit(
                `texto:${step.id}`,
                (d) => setStepText(d, step.id, e.target.value),
                true,
              )
            }
          />
        </Row>
      </>
    );
  }

  return (
    <>
      <h2 className="inspector__title">Marco</h2>
      <Row label="Tipo">
        <select
          value={step.kind}
          onChange={(e) =>
            edit("tipo de marco", (d) => {
              const block = locate(scriptOf(d).steps, step.id)?.step;
              if (block?.type === "block") block.kind = e.target.value as BlockKind;
            })
          }
        >
          {Object.entries(BLOCK_KIND_LABEL).map(([value, label]) => (
            <option key={value} value={value}>
              {label}
            </option>
          ))}
        </select>
      </Row>

      {step.branches.map((branch, index) => (
        <Row key={branch.id} label={index === 0 ? "Condicion" : "Si no"}>
          <input
            value={branch.label ?? ""}
            placeholder={index === 0 ? "hay stock" : "no hay stock"}
            onChange={(e) =>
              edit(
                `rama:${branch.id}`,
                (d) => setBranchLabel(d, branch.id, e.target.value),
                true,
              )
            }
          />
        </Row>
      ))}

      <Row label="Ramas">
        <div className="seq__buttons">
          <button
            type="button"
            onClick={() => edit("rama", (d) => addBranch(d, step.id))}
          >
            + Rama
          </button>
          {step.branches.length > 1 && (
            <button
              type="button"
              onClick={() =>
                edit("rama", (d) => {
                  const last = step.branches[step.branches.length - 1];
                  if (last) removeStep(d, last.id);
                })
              }
            >
              − Rama
            </button>
          )}
        </div>
      </Row>
    </>
  );
}

function Row({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="inspector__field">
      <span className="inspector__label">{label}</span>
      {children}
    </label>
  );
}
