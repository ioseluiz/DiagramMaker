import { useEffect } from "react";

import { GraphCanvas } from "@/canvas/graph/GraphCanvas";
import { LaneCanvas } from "@/canvas/lane/LaneCanvas";
import { selectDocument, useDocumentStore } from "@/store/documentStore";
import { CodePanel } from "./CodePanel";
import { CommandPalette } from "./CommandPalette";
import { Inspector } from "./Inspector";
import { RecoveryToast } from "./RecoveryToast";
import { StatusBar } from "./StatusBar";
import { Toolbar } from "./Toolbar";
import { useAutosave } from "./useAutosave";
import { useCodePanel } from "./useCodePanel";
import { useShortcuts } from "./useShortcuts";
import { useWindowTitle } from "./useWindowTitle";
import "./App.css";

export function App() {
  const doc = useDocumentStore(selectDocument);
  const newDocument = useDocumentStore((s) => s.newDocument);
  const loadRecents = useDocumentStore((s) => s.loadRecents);
  const checkRecovery = useDocumentStore((s) => s.checkRecovery);
  const codeOpen = useCodePanel((s) => s.open);

  useAutosave();
  useShortcuts();
  useWindowTitle();

  useEffect(() => {
    // Arrancar con un lienzo en blanco en vez de una pantalla vacia.
    if (!useDocumentStore.getState().history) {
      void newDocument("architecture");
    }
    void loadRecents();
    void checkRecovery();
  }, [checkRecovery, loadRecents, newDocument]);

  return (
    <div className="app-shell">
      <Toolbar />

      <div className="app-body" data-code={codeOpen || undefined}>
        {codeOpen && <CodePanel />}

        <main className="app-canvas">
          {doc && doc.kind !== "sequence" && <GraphCanvas document={doc} />}
          {doc?.kind === "sequence" && <LaneCanvas document={doc} />}
        </main>

        <Inspector />
      </div>

      <CommandPalette />

      <RecoveryToast />

      <StatusBar />
    </div>
  );
}
