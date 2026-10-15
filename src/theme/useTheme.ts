import { useCallback, useEffect, useState } from "react";

export type ThemePreference = "system" | "light" | "dark";

const STORAGE_KEY = "diagrammaker.theme";

function readStored(): ThemePreference {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === "light" || raw === "dark" || raw === "system") return raw;
  } catch {
    // localStorage puede lanzar (modo privado, politicas); el default basta.
  }
  return "system";
}

function apply(pref: ThemePreference): void {
  const root = document.documentElement;
  if (pref === "system") {
    root.removeAttribute("data-theme");
  } else {
    root.setAttribute("data-theme", pref);
  }
}

/**
 * Preferencia de tema persistida. `system` no estampa atributo alguno y deja
 * que `prefers-color-scheme` decida, que es lo que espera `theme.css`.
 */
export function useTheme(): {
  preference: ThemePreference;
  setPreference: (next: ThemePreference) => void;
} {
  const [preference, setPreferenceState] = useState<ThemePreference>(readStored);

  useEffect(() => {
    apply(preference);
  }, [preference]);

  const setPreference = useCallback((next: ThemePreference) => {
    setPreferenceState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // Persistir es una comodidad, no un requisito.
    }
  }, []);

  return { preference, setPreference };
}
