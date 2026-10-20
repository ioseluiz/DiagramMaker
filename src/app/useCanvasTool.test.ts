import { beforeEach, describe, expect, it } from "vitest";

import { useCanvasTool } from "./useCanvasTool";

beforeEach(() => {
  useCanvasTool.setState({ tool: "select" });
});

describe("useCanvasTool", () => {
  it("arranca con el puntero", () => {
    expect(useCanvasTool.getState().tool).toBe("select");
  });

  it("cambia de herramienta", () => {
    useCanvasTool.getState().setTool("hand");

    expect(useCanvasTool.getState().tool).toBe("hand");
  });
});
