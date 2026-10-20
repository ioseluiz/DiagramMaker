# Qué entiende cada idioma

El subconjunto que se soporta es una **decisión**, no un accidente. Este
documento dice dónde está la frontera, porque un importador que calla lo que
no entendió es peor que uno que no lo intenta.

Regla general, igual en los tres idiomas: **lo que no se entiende se reporta
con línea y columna, y la lectura sigue con el resto**. Nunca se adivina una
interpretación.

Qué parte de un documento llega a diagrama, y qué se decide por el camino,
está sostenido por pruebas de ida y vuelta: importar, generar y volver a
importar tiene que dar el mismo diagrama.

---

## DBML — entidad-relación

Sintaxis de [dbdiagram.io](https://dbdiagram.io).

**Entra**: `Table` con sus ajustes en línea (`pk`, `increment`, `not null`,
`unique`, `default`, `note`), `Ref` sueltas y en línea con los cuatro
operadores (`>`, `<`, `-`, `<>`), `Enum`, `TableGroup`, `Project` y notas.

**Sale**: el esquema completo, con alias y esquema de tabla si los había.

## Mermaid

### `erDiagram`

**Entra**: relaciones con los ocho símbolos de cardinalidad, identificadoras
(`--`) y no identificadoras (`..`), bloques de atributos con `PK` / `FK` /
`UK` y comentario, nombres entrecomillados, alias (`PEDIDO["Pedido"]`),
`title` y `direction`.

**Dos pérdidas, las dos a propósito:**

- Mermaid distingue «cero o uno» de «exactamente uno», y «cero o varios» de
  «uno o varios». El documento solo guarda `1` y `*`, porque la cardinalidad
  de un entidad-relación es en toda la aplicación una lista cerrada de cuatro
  casos —así el inspector no deja escribir un `N` que luego el generador de
  DBML no sabría leer—. Al volver a salir, `|o` se escribe `||` y `}|` se
  escribe `}o`.
- Mermaid exige `tipo nombre` en un atributo: una sola palabra rompe el dibujo
  entero. Una columna sin tipo sale como `sin_tipo`, que dice la verdad y se
  ve. No se deshace al volver a leer.

También: una relación de Mermaid une **entidades**, no columnas. Un esquema
que venga de DBML sabiendo de qué columna a qué columna va pierde ese dato al
escribirse en Mermaid; lo que sí se escribe es el `FK` del atributo.

### `flowchart`

**Entra**: las direcciones (`TD`, `TB`, `LR`, `RL`, `BT`), las formas por sus
delimitadores, `subgraph` anidados, los tipos de flecha y las etiquetas, y
`style` para el color de un nodo concreto.

**No entra**: `classDef` y `class`, porque los estilos por clase no tienen
dónde guardarse en el documento; `linkStyle` y `click`. Se avisa de los
cuatro y la lectura sigue.

### `classDiagram`

**Entra**: `class` con bloque o con miembros sueltos (`Pedido : +int id`),
visibilidad por sigla, tipo **delante** del nombre, `$` estático y `*`
abstracto al final, estereotipos `<<interface>>`, y las relaciones con su
dirección.

### `stateDiagram-v2`

**Entra**: `[*]` como arranque y salida —uno de cada por nivel—, estados
compuestos, `<<fork>>`, `<<join>>`, `<<choice>>`, descripciones y transiciones
con etiqueta.

### `sequenceDiagram`

**Entra**: `participant` y `actor` con alias, los tipos de mensaje,
`activate` / `deactivate` en su propia línea y la forma abreviada pegada a la
flecha (`A->>+B`, `B-->>-A`), notas, y los bloques
`alt` / `opt` / `loop` / `par` / `critical` / `break` con sus ramas.

## PlantUML

PlantUML no declara de qué diagrama se trata: todos empiezan por `@startuml`.
Se deduce del contenido con una regla decisiva, no estadística, en este orden:

1. Una palabra que solo existe en una secuencia (`participant`, `activate`,
   `alt`…) → secuencia.
2. Una que solo existe en clases (`class`, `abstract`, `enum`…) → clases.
3. `state` o un `[*]` → máquina de estados.
4. Una sentencia de actividad (`start`, `fork`, `if (...)`, una acción
   `:texto;`) → actividad.
5. `usecase` o una elipse `(Caso de uso)` → casos de uso.
6. Un `[Nombre]` o un bloque `package { }` → componentes.
7. Mensajes con etiqueta entre nombres sin declarar → secuencia.
8. En cualquier otro caso, componentes.

Quedan dos casos ambiguos y en los dos se elige componentes: `A --> B` a secas
y un `@startuml` que solo declare `interface`, palabra que los dos dialectos
usan.

### Componentes

**Entra**: las palabras de declaración (`component`, `rectangle`, `package`,
`node`, `cloud`, `folder`, `frame`, `card`, `database`, `queue`, `actor`,
`interface`…), las formas cortas `[Componente]`, bloques anidados, flechas con
pista de dirección (`-down->`), color (`-[#red]->`) y estilo (`-[dashed]->`),
cardinalidades entrecomilladas y colores de relleno.

### Casos de uso

Se leen con el mismo lector que los componentes, porque estructuralmente son
lo mismo. Lo propio son las formas cortas: `(Caso de uso)` y `:Actor:`.

### Clases

**Entra**: `class`, `abstract`, `interface`, `enum`, miembros con el tipo
**detrás** del nombre, `{static}` y `{abstract}`, estereotipos y las
relaciones con su dirección.

Los parámetros de una operación se guardan **tal como venían escritos**, sin
traducirlos al otro idioma. Un `int id` de Mermaid sigue siendo `int id`, y un
`id : int` de PlantUML sigue siendo `id : int`.

### Secuencia

**Entra**: `participant` y demás palabras de declaración con alias, los tipos
de mensaje, `activate` / `deactivate`, la forma abreviada `++` / `--` pegada a
un mensaje, notas y los bloques con sus ramas.

**No entra**: `autonumber`, ni la desactivación escrita sola en su línea
(`svc --`) —esa se escribe `deactivate svc`—. De las dos se avisa.

### Estados

**Entra**: `[*]`, estados compuestos, `<<fork>>` / `<<join>>` / `<<choice>>` /
`<<start>>` / `<<end>>`, descripciones acumulativas (dos líneas sobre el mismo
estado son dos renglones) y transiciones con etiqueta.

**No entra**: las regiones concurrentes (`--` dentro de un compuesto). Se
avisa y lo de dentro se queda en el mismo estado.

### Actividad

Es el único dialecto que no se escribe como una lista de relaciones: es una
secuencia de sentencias donde el orden del texto dice por dónde sigue el
flujo.

**Entra**: `start` / `stop` / `end`, acciones de una o varias líneas,
`if` / `elseif` / `else` / `endif`, `while` / `endwhile`,
`repeat` / `repeat while`, `fork` / `fork again` / `end fork`, `end merge`,
`split` y `partition`.

La diferencia entre los dos bucles se respeta donde importa: `while` pregunta
antes, y `repeat` vuelve al primer paso del cuerpo, que por eso se ejecuta al
menos una vez.

**No entra**: la sintaxis antigua (`(*) --> "algo"`), que es otro idioma;
`detach` y `kill`; `backward`; y las etiquetas sueltas de flecha (`-> texto;`).

**Al generar**, dos cosas no vuelven iguales y se dicen en vez de disimularse:

- Dónde cerraba un `if` cuya otra rama acaba en `stop`. Si una rama se corta,
  las dos no se juntan en ningún sitio y el grafo ya no guarda ese punto: lo
  que seguía se escribe dentro de la rama viva. Es el mismo flujo, con más
  sangría.
- Un diagrama dibujado a mano puede no ser expresable —PlantUML no tiene cómo
  escribir un salto al centro de un bloque—. Antes de cerrar, el generador
  compara lo escrito con las relaciones del documento y **nombra una a una las
  que no han cabido**, en comentarios. Escribir un flujo parecido y callarse
  sería el peor fallo posible.

---

## Lo que ningún idioma conserva

- **Las posiciones.** Nada de lo que se importa trae geometría; la pone el
  auto-layout. Si mueves las cajas y vuelves a generar código, el código no
  guarda dónde estaban.
- **El color del trazo y del texto** de una relación en PlantUML. El de
  relleno sí (`#LightBlue`).
- **La dirección del reparto**, salvo las dos que PlantUML sabe escribir:
  `Left` sale como `left to right` y `Up` como `top to bottom`.
