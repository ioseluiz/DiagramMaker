import { invoke } from "@tauri-apps/api/core";
import type { AppInfo } from "./generated/AppInfo";

export type { AppInfo };

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
 * Unico punto por el que el frontend llama a Rust. Mantener todas las llamadas
 * aqui evita que los nombres de comando se dispersen por los componentes.
 */
export function appInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("app_info");
}
