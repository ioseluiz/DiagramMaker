/**
 * Estilos por nivel del modelo C4.
 *
 * C4 no es otro tipo de diagrama: es una convencion sobre diagramas de
 * arquitectura corrientes. Lo unico que aporta al dibujo es un color por
 * nivel, y son colores concretos —los de la notacion, los mismos que usan
 * Structurizr y C4-PlantUML— no una eleccion de gusto. Un diagrama C4 se
 * reconoce justamente por ellos.
 *
 * Por eso estos rellenos son solidos y no tintes translucidos como los del
 * resto de la paleta, y por eso cada nivel trae su color de texto: a esa
 * saturacion el texto del tema no se leeria.
 *
 * El nivel no se guarda en el documento. Es un atajo que escribe tres
 * propiedades de estilo a la vez, y se deduce de vuelta del relleno. Asi un
 * `.dgm` sigue describiendo solo lo que se dibuja, sin un campo que pudiera
 * contradecirlo.
 */

export interface C4Level {
  id: string;
  name: string;
  /** Que representa el nivel. Se muestra como ayuda. */
  hint: string;
  fill: string;
  stroke: string;
  textColor: string;
}

export const C4_LEVELS: readonly C4Level[] = [
  {
    id: "person",
    name: "Persona",
    hint: "Quien usa el sistema",
    fill: "#08427B",
    stroke: "#073B6F",
    textColor: "#FFFFFF",
  },
  {
    id: "system",
    name: "Sistema",
    hint: "El sistema del que trata el diagrama",
    fill: "#1168BD",
    stroke: "#0B4884",
    textColor: "#FFFFFF",
  },
  {
    id: "external",
    name: "Externo",
    hint: "Sistema o persona fuera del alcance",
    fill: "#999999",
    stroke: "#6B6B6B",
    textColor: "#FFFFFF",
  },
  {
    id: "container",
    name: "Contenedor",
    hint: "Aplicacion o almacen que se despliega por separado",
    fill: "#438DD5",
    stroke: "#2E6295",
    textColor: "#FFFFFF",
  },
  {
    id: "component",
    name: "Componente",
    hint: "Pieza de dentro de un contenedor",
    fill: "#85BBF0",
    stroke: "#5D82A8",
    // La notacion pone texto blanco tambien aqui, pero sobre este azul tan
    // claro no se lee. Se aparta de ella a proposito: el diagrama se sigue
    // reconociendo por el relleno, y el texto hay que poder leerlo.
    textColor: "#0B2A45",
  },
];

export function c4LevelById(id: string): C4Level | undefined {
  return C4_LEVELS.find((level) => level.id === id);
}

/** A que nivel corresponde un relleno, si es que corresponde a alguno. */
export function c4LevelOf(fill: string | null | undefined): C4Level | undefined {
  return C4_LEVELS.find((level) => level.fill === fill);
}
