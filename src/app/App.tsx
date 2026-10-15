import { useEffect, useState } from "react";
import { appInfo } from "@/model/api";
import type { AppInfo } from "@/model/api";
import { useTheme, type ThemePreference } from "@/theme/useTheme";
import "./App.css";

const THEME_LABEL: Record<ThemePreference, string> = {
  system: "Sistema",
  light: "Claro",
  dark: "Oscuro",
};

export function App() {
  const { preference, setPreference } = useTheme();
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    appInfo()
      .then(setInfo)
      .catch((e: unknown) => setError(String(e)));
  }, []);

  return (
    <div className="app-shell">
      <header className="app-bar">
        <span className="app-bar__title">DiagramMaker</span>
        <span className="app-bar__spacer" />
        <label className="app-bar__field">
          Tema
          <select
            value={preference}
            onChange={(e) => setPreference(e.target.value as ThemePreference)}
          >
            {(["system", "light", "dark"] as const).map((p) => (
              <option key={p} value={p}>
                {THEME_LABEL[p]}
              </option>
            ))}
          </select>
        </label>
      </header>

      <main className="app-main">
        <div className="placeholder">
          <h1>Andamiaje listo</h1>
          <p>
            El lienzo interactivo llega en la fase F1. Este shell existe para verificar
            que Tauri, el puente IPC y el tema funcionan de extremo a extremo.
          </p>
          <dl className="kv">
            <dt>Backend</dt>
            <dd className="selectable">
              {error
                ? `error: ${error}`
                : info
                  ? `${info.name} ${info.version} · esquema de documento v${info.schemaVersion}`
                  : "consultando…"}
            </dd>
          </dl>
        </div>
      </main>

      <footer className="status-bar">
        <span>Sin documento</span>
        <span className="app-bar__spacer" />
        <span>F0 · andamiaje</span>
      </footer>
    </div>
  );
}
