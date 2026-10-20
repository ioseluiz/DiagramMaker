import type { Template } from "@/model/generated/Template";
import * as api from "@/model/api";
import { applyAutoLayout } from "@/layout/autoLayout";
import { useDocumentStore } from "@/store/documentStore";

/**
 * Abre un diagrama de arranque.
 *
 * Vive fuera del dialogo porque no es parte de pintarlo: es una operacion
 * sobre el documento, con su reloj y su llamada al backend, y asi puede
 * probarse sin montar nada.
 *
 * La plantilla llega como codigo y entra por el importador, igual que si se
 * hubiera pegado a mano. No hay un segundo camino que mantener, y lo que se
 * abre es un documento corriente desde el primer momento.
 */
export async function openTemplate(template: Template): Promise<void> {
  const result = await api.importCode(template.language, template.source, Date.now());

  // Llega sin geometria, como todo lo importado: colocarlo es cosa de ELK.
  const document = result.document;
  await applyAutoLayout(document);

  // Los diagnosticos se pasan igualmente. Las plantillas se prueban en el
  // backend y no deberian traer ninguno; si lo traen, es un fallo que
  // conviene ver en el panel y no esconder.
  useDocumentStore.getState().replaceDocument(document, result.diagnostics);
}
