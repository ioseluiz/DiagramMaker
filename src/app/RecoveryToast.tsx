import { useDocumentStore } from "@/store/documentStore";
import { basename, relativeTime } from "./time";

/**
 * Aviso de copia de recuperacion pendiente.
 *
 * Flota sobre el lienzo en lugar de ocupar una franja propia: el aviso importa,
 * pero no tanto como para reorganizar la ventana cada vez que se arranca.
 *
 * Nunca restaura solo. El documento en pantalla puede ser mas reciente que la
 * copia, y sobrescribirlo sin preguntar seria peor que el problema que
 * resuelve.
 *
 * Cerrar con la × **no borra nada**: solo esconde el aviso. La copia se limpia
 * sola en cuanto se guarda, y mientras tanto el autoguardado la va
 * reemplazando por el trabajo actual. Hacer destructivo el boton de cerrar
 * seria un mal reparto de consecuencias en una funcion que existe justo para
 * no perder trabajo.
 */
export function RecoveryToast() {
  const recovery = useDocumentStore((s) => s.recovery);
  const restore = useDocumentStore((s) => s.restoreRecovery);
  const discard = useDocumentStore((s) => s.discardRecovery);
  const dismiss = useDocumentStore((s) => s.dismissRecovery);

  if (!recovery) return null;

  const origin = recovery.path ? basename(recovery.path) : "sin guardar";

  return (
    <div className="recovery-toast" role="status">
      <div className="recovery-toast__head">
        <strong className="recovery-toast__title">Cambios sin guardar</strong>
        <button
          type="button"
          className="recovery-toast__close"
          onClick={dismiss}
          title="Cerrar. La copia se descarta sola al guardar."
          aria-label="Cerrar aviso"
        >
          ×
        </button>
      </div>

      <p className="recovery-toast__detail">
        {relativeTime(recovery.savedAt)} · {origin}
      </p>

      <div className="recovery-toast__actions">
        <button type="button" onClick={() => void discard()}>
          Descartar
        </button>
        <button
          type="button"
          className="recovery-toast__primary"
          onClick={restore}
          autoFocus
        >
          Recuperar
        </button>
      </div>
    </div>
  );
}
