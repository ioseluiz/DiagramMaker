import { useEffect, useState } from "react";

import type { AppInfo } from "@/model/generated/AppInfo";
import * as api from "@/model/api";

/**
 * Identidad de la aplicacion: nombre, version y version del esquema.
 *
 * Se pide una sola vez por sesion y se comparte: no cambia mientras el
 * programa esta abierto, y cada componente que la necesite no tiene por que
 * cruzar el puente IPC por su cuenta.
 */
let pending: Promise<AppInfo> | null = null;

function load(): Promise<AppInfo> {
  pending ??= api.appInfo();
  return pending;
}

export function useAppInfo(): AppInfo | null {
  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    let alive = true;

    load()
      .then((value) => {
        if (alive) setInfo(value);
      })
      .catch(() => {
        // Sin esta informacion la aplicacion sigue siendo usable; lo unico
        // que se pierde es el numero de version en la barra de estado.
      });

    return () => {
      alive = false;
    };
  }, []);

  return info;
}
