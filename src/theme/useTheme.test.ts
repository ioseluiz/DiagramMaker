import { act, renderHook } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { useTheme } from "./useTheme";

describe("useTheme", () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("arranca en 'system' y no estampa data-theme", () => {
    const { result } = renderHook(() => useTheme());

    expect(result.current.preference).toBe("system");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("estampa data-theme al elegir un tema explicito", () => {
    const { result } = renderHook(() => useTheme());

    act(() => result.current.setPreference("dark"));

    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(localStorage.getItem("diagrammaker.theme")).toBe("dark");
  });

  it("vuelve a quitar data-theme al regresar a 'system'", () => {
    const { result } = renderHook(() => useTheme());

    act(() => result.current.setPreference("light"));
    act(() => result.current.setPreference("system"));

    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("restaura la preferencia guardada", () => {
    localStorage.setItem("diagrammaker.theme", "dark");

    const { result } = renderHook(() => useTheme());

    expect(result.current.preference).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });
});
