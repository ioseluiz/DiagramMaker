import { useDocumentStore } from "@/store/documentStore";

/**
 * Aviso de copia de recuperacion pendiente.
 *
 * Nunca restaura solo: el documento en pantalla puede ser mas reciente que la
 * copia, y sobrescribirlo sin preguntar seria peor que el problema que
 * resuelve.
 */
export function RecoveryBanner() {
  const recovery = useDocumentStore((s) => s.recovery);
  const restore = useDocumentStore((s) => s.restoreRecovery);
  const discard = useDocumentStore((s) => s.discardRecovery);

  if (!recovery) return null;

  const when = new Date(recovery.savedAt).toLocaleString();
  const name = recovery.path?.split(/[\\/]/).pop();

  return (
    <div className="recovery-banner" role="status">
      <span>
        Hay cambios sin guardar de {when}
        {name ? ` en ${name}` : " de un diagrama sin guardar"}.
      </span>
      <button type="button" className="recovery-banner__primary" onClick={restore}>
        Recuperar
      </button>
      <button type="button" onClick={() => void discard()}>
        Descartar
      </button>
    </div>
  );
}
