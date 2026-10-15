import { produce } from "immer";
import { describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { Step } from "@/model/generated/Step";
import { flatten, locate } from "@/model/sequence";
import {
  addBranch,
  addMessage,
  addParticipant,
  addSequenceBlock,
  addSequenceNote,
  moveParticipant,
  moveStep,
  pruneScript,
  removeElements,
  removeStep,
  setBranchLabel,
  setMessageActivation,
  setMessageEndpoint,
  setMessageKind,
  setStepText,
} from "./operations";

function emptySequence(): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "sequence",
    meta: { title: "Prueba", created: 0, modified: 0 },
    nodes: [],
    edges: [],
    layout: "auto",
    direction: "right",
    script: { steps: [] },
  };
}

function run(doc: Document, fn: (draft: Document) => void): Document {
  // Igual que hace el store: lo que la receta devuelva no llega a immer.
  return produce(doc, (draft) => {
    fn(draft);
  });
}

/** Documento con dos participantes y sus ids. */
function conDos(): { doc: Document; a: string; b: string } {
  let a = "";
  let b = "";
  const doc = run(emptySequence(), (d) => {
    a = addParticipant(d, "Cliente");
    b = addParticipant(d, "API");
  });
  return { doc, a, b };
}

const steps = (doc: Document): Step[] => doc.script?.steps ?? [];

describe("participantes", () => {
  it("un participante nuevo no trae geometria", () => {
    // En una secuencia la posicion sale del orden, no de coordenadas.
    const { doc } = conDos();

    expect(doc.nodes).toHaveLength(2);
    expect(doc.nodes.every((n) => n.geometry === undefined)).toBe(true);
  });

  it("el orden de los nodos es el orden de izquierda a derecha", () => {
    const { doc: base, b } = conDos();

    const doc = run(base, (d) => moveParticipant(d, b, -1));

    expect(doc.nodes.map((n) => n.label)).toEqual(["API", "Cliente"]);
  });

  it("no se sale de los extremos al reordenar", () => {
    const { doc: base, a } = conDos();

    const doc = run(base, (d) => moveParticipant(d, a, -1));

    expect(doc.nodes.map((n) => n.label)).toEqual(["Cliente", "API"]);
  });
});

describe("mensajes", () => {
  it("un mensaje se anade al final del guion", () => {
    const { doc: base, a, b } = conDos();

    const doc = run(base, (d) => {
      addMessage(d, { from: a, to: b, text: "pedir" });
      addMessage(d, { from: b, to: a, kind: "reply", text: "dar" });
    });

    expect(steps(doc)).toHaveLength(2);
    expect(steps(doc)[1]).toMatchObject({ kind: "reply", text: "dar" });
  });

  it("se puede insertar detras de un paso concreto", () => {
    const { doc: base, a, b } = conDos();

    let primero = "";
    const conUno = run(base, (d) => {
      primero = addMessage(d, { from: a, to: b, text: "uno" });
      addMessage(d, { from: a, to: b, text: "tres" });
    });

    const doc = run(conUno, (d) => {
      addMessage(d, { from: a, to: b, text: "dos", after: primero });
    });

    expect(steps(doc).map((s) => (s.type === "message" ? s.text : ""))).toEqual([
      "uno",
      "dos",
      "tres",
    ]);
  });

  it("cambiar un extremo lo cambia en el guion", () => {
    const { doc: base, a, b } = conDos();
    let id = "";
    const conMensaje = run(base, (d) => {
      id = addMessage(d, { from: a, to: b });
    });

    const doc = run(conMensaje, (d) => setMessageEndpoint(d, id, "to", a));
    const paso = locate(steps(doc), id)!.step;

    expect(paso.type === "message" && paso.to).toBe(a);
  });

  it("el tipo, el texto y la activacion se editan en su sitio", () => {
    const { doc: base, a, b } = conDos();
    let id = "";
    const conMensaje = run(base, (d) => {
      id = addMessage(d, { from: a, to: b });
    });

    const doc = run(conMensaje, (d) => {
      setMessageKind(d, id, "async");
      setStepText(d, id, "en cola");
      setMessageActivation(d, id, "activate", true);
    });

    expect(locate(steps(doc), id)!.step).toMatchObject({
      kind: "async",
      text: "en cola",
      activate: true,
    });
  });
});

describe("marcos", () => {
  /** Un `alt` con un mensaje dentro de su primera rama. */
  function conMarco() {
    const { doc: base, a, b } = conDos();
    let bloque = "";
    let dentro = "";

    const doc = run(base, (d) => {
      bloque = addSequenceBlock(d, "alt", "hay stock");
      const rama = d.script!.steps[0];
      if (rama?.type === "block") {
        dentro = addMessage(d, { from: a, to: b, text: "reservar" });
        // `addMessage` lo deja al final del guion; moverlo dentro de la rama
        // es lo que hace el panel al insertar detras del marco.
        const paso = d.script!.steps.pop()!;
        rama.branches[0]!.steps.push(paso);
      }
    });

    return { doc, bloque, dentro, a, b };
  }

  it("un marco nace con una rama", () => {
    const { doc, bloque } = conMarco();
    const paso = locate(steps(doc), bloque)!.step;

    expect(paso.type === "block" && paso.branches).toHaveLength(1);
    expect(paso.type === "block" && paso.branches[0]!.label).toBe("hay stock");
  });

  it("el recorrido plano entra en las ramas e indica la profundidad", () => {
    const { doc } = conMarco();
    const plano = flatten(steps(doc));

    expect(plano.map((e) => e.depth)).toEqual([0, 1]);
  });

  it("anadir una rama la deja detras de la primera", () => {
    const { doc: base, bloque } = conMarco();

    const doc = run(base, (d) => addBranch(d, bloque, "no hay stock"));
    const paso = locate(steps(doc), bloque)!.step;

    expect(paso.type === "block" && paso.branches.map((b) => b.label)).toEqual([
      "hay stock",
      "no hay stock",
    ]);
  });

  it("no se borra la ultima rama: dejaria el marco sin cuerpo", () => {
    const { doc: base, bloque } = conMarco();
    const paso = locate(steps(base), bloque)!.step;
    const rama = paso.type === "block" ? paso.branches[0]!.id : "";

    const doc = run(base, (d) => removeStep(d, rama));
    const despues = locate(steps(doc), bloque)!.step;

    expect(despues.type === "block" && despues.branches).toHaveLength(1);
  });

  it("la condicion de una rama se edita por su id", () => {
    const { doc: base, bloque } = conMarco();
    const paso = locate(steps(base), bloque)!.step;
    const rama = paso.type === "block" ? paso.branches[0]!.id : "";

    const doc = run(base, (d) => setBranchLabel(d, rama, "otra cosa"));
    const despues = locate(steps(doc), bloque)!.step;

    expect(despues.type === "block" && despues.branches[0]!.label).toBe("otra cosa");
  });

  it("un paso se mueve entre sus hermanos, no fuera de su marco", () => {
    const { doc: base, dentro } = conMarco();

    // Esta solo dentro de la rama: subirlo no debe sacarlo al nivel de fuera.
    const doc = run(base, (d) => moveStep(d, dentro, -1));

    expect(steps(doc)).toHaveLength(1);
    expect(flatten(steps(doc)).find((e) => e.step.id === dentro)?.depth).toBe(1);
  });

  it("borrar un marco se lleva lo que contiene", () => {
    const { doc: base, bloque } = conMarco();

    const doc = run(base, (d) => removeStep(d, bloque));

    expect(flatten(steps(doc))).toHaveLength(0);
  });
});

describe("limpieza al borrar un participante", () => {
  it("se van los mensajes que lo tocaban", () => {
    const { doc: base, a, b } = conDos();
    const conMensajes = run(base, (d) => {
      addMessage(d, { from: a, to: b, text: "cruzado" });
      addMessage(d, { from: a, to: a, text: "solo" });
    });

    const doc = run(conMensajes, (d) => removeElements(d, [b]));

    expect(doc.nodes).toHaveLength(1);
    expect(steps(doc).map((s) => (s.type === "message" ? s.text : ""))).toEqual([
      "solo",
    ]);
  });

  it("se van las notas que lo mencionaban", () => {
    const { doc: base, a, b } = conDos();
    const conNota = run(base, (d) => {
      addSequenceNote(d, [a, b], "over", "las dos");
    });

    const doc = run(conNota, (d) => removeElements(d, [b]));

    expect(steps(doc)).toHaveLength(0);
  });

  it("la limpieza entra dentro de los marcos", () => {
    const { doc: base, a, b } = conDos();

    const conMarco = run(base, (d) => {
      addSequenceBlock(d, "opt", "quiza");
      const bloque = d.script!.steps[0];
      if (bloque?.type === "block") {
        bloque.branches[0]!.steps.push({
          type: "message",
          id: "m1",
          from: a,
          to: b,
          kind: "sync",
          text: "dentro",
          activate: false,
          deactivate: false,
        });
      }
    });

    const doc = run(conMarco, (d) => removeElements(d, [b]));
    const bloque = steps(doc)[0];

    expect(bloque?.type === "block" && bloque.branches[0]!.steps).toHaveLength(0);
  });

  it("un documento sin guion no se rompe al limpiar", () => {
    const sinGuion: Document = { ...emptySequence(), script: undefined };

    expect(() => run(sinGuion, (d) => pruneScript(d))).not.toThrow();
  });
});
