import { describe, expect, it } from "vitest";

import {
  HISTORY_LIMIT,
  canRedo,
  canUndo,
  commit,
  commitCoalescing,
  initHistory,
  redo,
  undo,
} from "./history";

interface Model {
  count: number;
  items: string[];
}

const start = (): ReturnType<typeof initHistory<Model>> =>
  initHistory<Model>({ count: 0, items: [] });

describe("history", () => {
  it("arranca sin nada que deshacer ni rehacer", () => {
    const h = start();

    expect(canUndo(h)).toBe(false);
    expect(canRedo(h)).toBe(false);
  });

  it("commit aplica la receta y habilita deshacer", () => {
    const h = commit(start(), "inc", (d) => {
      d.count = 1;
    });

    expect(h.present.count).toBe(1);
    expect(canUndo(h)).toBe(true);
  });

  it("no toca el estado original: el presente anterior sigue intacto", () => {
    const before = start();
    const after = commit(before, "inc", (d) => {
      d.count = 99;
    });

    expect(before.present.count).toBe(0);
    expect(after.present).not.toBe(before.present);
  });

  it("una receta sin efecto devuelve el mismo historial", () => {
    const before = commit(start(), "inc", (d) => {
      d.count = 1;
    });
    const after = commit(before, "nada", (d) => {
      d.count = 1;
    });

    expect(after).toBe(before);
    expect(after.past).toHaveLength(1);
  });

  it("deshacer vuelve al valor previo y rehacer lo restaura", () => {
    let h = commit(start(), "inc", (d) => {
      d.count = 5;
    });

    h = undo(h);
    expect(h.present.count).toBe(0);
    expect(canRedo(h)).toBe(true);

    h = redo(h);
    expect(h.present.count).toBe(5);
    expect(canRedo(h)).toBe(false);
  });

  it("deshace varios pasos en orden inverso", () => {
    let h = start();
    for (const item of ["a", "b", "c"]) {
      h = commit(h, `add:${item}`, (d) => {
        d.items.push(item);
      });
    }

    h = undo(h);
    expect(h.present.items).toEqual(["a", "b"]);

    h = undo(h);
    expect(h.present.items).toEqual(["a"]);

    h = redo(h);
    expect(h.present.items).toEqual(["a", "b"]);
  });

  it("un commit nuevo descarta el futuro", () => {
    let h = commit(start(), "a", (d) => {
      d.count = 1;
    });
    h = undo(h);
    expect(canRedo(h)).toBe(true);

    h = commit(h, "b", (d) => {
      d.count = 2;
    });

    expect(canRedo(h)).toBe(false);
    expect(h.present.count).toBe(2);
  });

  it("deshacer sin pasado y rehacer sin futuro no hacen nada", () => {
    const h = start();

    expect(undo(h)).toBe(h);
    expect(redo(h)).toBe(h);
  });

  it("poda el historial al llegar al limite", () => {
    let h = start();
    for (let i = 0; i < HISTORY_LIMIT + 25; i++) {
      h = commit(h, `inc:${i}`, (d) => {
        d.count = i + 1;
      });
    }

    expect(h.past).toHaveLength(HISTORY_LIMIT);
    expect(h.present.count).toBe(HISTORY_LIMIT + 25);
  });

  describe("commitCoalescing", () => {
    it("fusiona pasos consecutivos con la misma etiqueta", () => {
      let h = start();
      for (const value of [1, 2, 3]) {
        h = commitCoalescing(h, "move:n1", (d) => {
          d.count = value;
        });
      }

      expect(h.past).toHaveLength(1);
      expect(h.present.count).toBe(3);
    });

    it("un solo deshacer revierte todo el arrastre fusionado", () => {
      let h = commit(start(), "crear", (d) => {
        d.count = 10;
      });

      for (const value of [11, 12, 13]) {
        h = commitCoalescing(h, "move:n1", (d) => {
          d.count = value;
        });
      }

      h = undo(h);

      expect(h.present.count).toBe(10);
      expect(canUndo(h)).toBe(true);
    });

    it("rehacer tras un arrastre fusionado devuelve el valor final", () => {
      let h = start();
      for (const value of [1, 2, 3]) {
        h = commitCoalescing(h, "move:n1", (d) => {
          d.count = value;
        });
      }

      h = redo(undo(h));

      expect(h.present.count).toBe(3);
    });

    it("no fusiona pasos con etiquetas distintas", () => {
      let h = commitCoalescing(start(), "move:n1", (d) => {
        d.count = 1;
      });
      h = commitCoalescing(h, "move:n2", (d) => {
        d.count = 2;
      });

      expect(h.past).toHaveLength(2);
    });

    it("no fusiona si entre medias hubo otra operacion", () => {
      let h = commitCoalescing(start(), "move:n1", (d) => {
        d.count = 1;
      });
      h = commit(h, "borrar", (d) => {
        d.items.push("x");
      });
      h = commitCoalescing(h, "move:n1", (d) => {
        d.count = 2;
      });

      expect(h.past).toHaveLength(3);
    });
  });
});
