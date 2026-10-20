import type { ClassBox } from "./generated/ClassBox";
import type { ClassMember } from "./generated/ClassMember";
import type { Node } from "./generated/Node";
import type { Visibility } from "./generated/Visibility";
import { measureWidth } from "@/canvas/text";
import {
  CLASS_MEMBER_FONT_SIZE,
  CLASS_NAME_FONT_SIZE,
  CLASS_PADDING,
  classHeight,
} from "./classMetrics";

/**
 * Caja de un diagrama de clases.
 *
 * Espejo de `src-tauri/src/model/class_box.rs`. Lo que de verdad importa que
 * coincida es `memberText`: el texto de un miembro se compone aqui para el
 * lienzo y alli para la exportacion, y un espacio de diferencia solo se
 * notaria comparando el PNG con la pantalla.
 * `src-tauri/tests/golden/class-members.json` lo vigila.
 */

/** El signo con el que UML escribe cada visibilidad. */
export const SIGIL: Record<Visibility, string> = {
  public: "+",
  private: "-",
  protected: "#",
  package: "~",
};

/** Nombre de cada visibilidad para la interfaz. */
export const VISIBILITY_LABEL: Record<Visibility, string> = {
  public: "Publica  (+)",
  private: "Privada  (−)",
  protected: "Protegida  (#)",
  package: "De paquete  (~)",
};

/** Que visibilidad declara un signo, si es que declara alguna. */
export function visibilityOf(sigil: string): Visibility | undefined {
  const found = Object.entries(SIGIL).find(([, value]) => value === sigil);
  return found?.[0] as Visibility | undefined;
}

/**
 * Como se escribe un miembro dentro de la caja.
 *
 * Una operacion lleva parentesis siempre, aunque no reciba nada; un atributo
 * no los lleva nunca. Es lo unico que los distingue al leerlos.
 */
export function memberText(member: ClassMember, isOperation: boolean): string {
  let out = `${SIGIL[member.visibility]} ${member.name}`;

  if (isOperation) out += `(${member.parameters})`;
  if (member.typeName) out += `: ${member.typeName}`;

  return out;
}

/** Caja de clase de un nodo, si lo es. */
export function classBoxOf(node: Node): ClassBox | null {
  return node.data.type === "class" ? node.data : null;
}

/** `true` si hay que hacer sitio para el estereotipo encima del nombre. */
export function hasStereotype(box: ClassBox): boolean {
  return (box.stereotype ?? "").trim().length > 0;
}

/**
 * Tamano que necesita una caja de clase.
 *
 * El alto es derivado: tres compartimentos y una fila por miembro. El ancho
 * sale de medir el texto mas largo, que es cosa del navegador.
 */
export function classSize(node: Node): { width: number; height: number } {
  const box = classBoxOf(node);
  if (!box) return { width: 180, height: 80 };

  const widest = Math.max(
    measureWidth(node.label, CLASS_NAME_FONT_SIZE, true),
    ...box.attributes.map((m) =>
      measureWidth(memberText(m, false), CLASS_MEMBER_FONT_SIZE),
    ),
    ...box.operations.map((m) =>
      measureWidth(memberText(m, true), CLASS_MEMBER_FONT_SIZE),
    ),
  );

  return {
    width: Math.max(160, Math.ceil(widest + CLASS_PADDING * 2)),
    height: classHeight(
      hasStereotype(box),
      box.attributes.length,
      box.operations.length,
    ),
  };
}
