import { useEffect } from "react";

import { useDocumentStore } from "@/store/documentStore";

/** Cada cuanto se vuelca la copia de recuperacion mientras hay cambios. */
export const AUTOSAVE_INTERVAL_MS = 10_000;

/**
 * Autoguardado de recuperacion.
 *
 * Escribe a un archivo aparte en el directorio de datos de la aplicacion,
 * **nunca** al `.dgm` del usuario: guardar sobre su archivo sin que lo haya
 * pedido convertiria un error de edicion en una perdida de datos.
 *
 * Tambien vuelca al cerrar la ventana, que es justo cuando mas falta hace.
 */
export function useAutosave() {
  const autosave = useDocumentStore((s) => s.autosave);

  useEffect(() => {
    const timer = setInterval(() => void autosave(), AUTOSAVE_INTERVAL_MS);

    const onLeave = () => void autosave();
    window.addEventListener("beforeunload", onLeave);

    return () => {
      clearInterval(timer);
      window.removeEventListener("beforeunload", onLeave);
    };
  }, [autosave]);
}
