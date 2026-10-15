import { open, save } from "@tauri-apps/plugin-dialog";

const FILTERS = [{ name: "Diagrama de DiagramMaker", extensions: ["dgm"] }];

export async function pickOpenPath(): Promise<string | null> {
  const picked = await open({ multiple: false, directory: false, filters: FILTERS });
  return typeof picked === "string" ? picked : null;
}

export async function pickSavePath(suggested: string): Promise<string | null> {
  return await save({ defaultPath: `${sanitize(suggested)}.dgm`, filters: FILTERS });
}

/** Deja el titulo del diagrama utilizable como nombre de archivo. */
function sanitize(name: string): string {
  const cleaned = name.replace(/[\\/:*?"<>|]/g, "").trim();
  return cleaned || "diagrama";
}
