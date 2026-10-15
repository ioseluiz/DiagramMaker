import { invoke } from "@tauri-apps/api/core";

import type { AppInfo } from "./generated/AppInfo";
import type { DiagramKind } from "./generated/DiagramKind";
import type { Document } from "./generated/Document";
import type { RecentFile } from "./generated/RecentFile";
import type { Recovery } from "./generated/Recovery";
import type { Report } from "./generated/Report";

export type { AppInfo, DiagramKind, Document, RecentFile, Recovery, Report };

/**
 * Forma de los errores que devuelve el backend (ver `src-tauri/src/error.rs`).
 */
export interface AppError {
  kind: "io" | "invalidDocument" | "unsupported";
  message: string;
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as AppError).kind === "string" &&
    typeof (value as AppError).message === "string"
  );
}

/**
 * Unico punto por el que el frontend llama a Rust. Concentrar aqui los nombres
 * de comando evita que se dispersen por los componentes y deja un solo sitio
 * que mockear en las pruebas.
 */

export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}

export function documentNew(kind: DiagramKind, nowMs: number): Promise<Document> {
  return invoke<Document>("document_new", { kind, nowMs });
}

export function documentOpen(path: string): Promise<Document> {
  return invoke<Document>("document_open", { path });
}

export function documentSave(path: string, document: Document): Promise<void> {
  return invoke<void>("document_save", { path, document });
}

export function documentValidate(document: Document): Promise<Report> {
  return invoke<Report>("document_validate", { document });
}

/* --- Espacio de trabajo ------------------------------------------------- */

export function recentList(): Promise<RecentFile[]> {
  return invoke<RecentFile[]>("recent_list");
}

export function recentRemember(path: string): Promise<RecentFile[]> {
  return invoke<RecentFile[]>("recent_remember", { path });
}

export function recentForget(path: string): Promise<RecentFile[]> {
  return invoke<RecentFile[]>("recent_forget", { path });
}

export function autosaveWrite(
  document: Document,
  path: string | null,
  nowMs: number,
): Promise<void> {
  return invoke<void>("autosave_write", { document, path, nowMs });
}

export function autosaveRead(): Promise<Recovery | null> {
  return invoke<Recovery | null>("autosave_read");
}

export function autosaveClear(): Promise<void> {
  return invoke<void>("autosave_clear");
}
