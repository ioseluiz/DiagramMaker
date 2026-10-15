import { describe, expect, it } from "vitest";

import { decodeHandle, encodeHandle } from "./handles";

describe("encodeHandle / decodeHandle", () => {
  it("un lado suelto se codifica como su nombre", () => {
    expect(encodeHandle("right")).toBe("right");
    expect(decodeHandle("right")).toEqual({ side: "right" });
  });

  it("una columna lleva su id delante", () => {
    expect(encodeHandle("left", "c_a1b2")).toBe("c_a1b2@left");
    expect(decodeHandle("c_a1b2@left")).toEqual({ side: "left", column: "c_a1b2" });
  });

  it("la ida y vuelta conserva ambos casos", () => {
    for (const column of [undefined, "c_x", "t_users.user_id"]) {
      for (const side of ["top", "right", "bottom", "left"] as const) {
        expect(decodeHandle(encodeHandle(side, column))).toEqual(
          column ? { side, column } : { side },
        );
      }
    }
  });

  it("devuelve null ante basura", () => {
    expect(decodeHandle(null)).toBeNull();
    expect(decodeHandle(undefined)).toBeNull();
    expect(decodeHandle("")).toBeNull();
    expect(decodeHandle("centro")).toBeNull();
    expect(decodeHandle("@right")).toBeNull();
    expect(decodeHandle("c_x@centro")).toBeNull();
  });
});
