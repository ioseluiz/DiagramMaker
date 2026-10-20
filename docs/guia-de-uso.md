# Guía de uso

DiagramMaker produce los seis tipos de diagrama que aparecen a diario en
documentación técnica. Se puede llegar a un diagrama por dos caminos, y los
dos acaban en el mismo sitio: un documento editable.

- **Dibujando** en el lienzo.
- **Pegando código** PlantUML, Mermaid o DBML.

Lo importado no es una imagen: es un diagrama nativo que se mueve, se cambia
y se vuelve a exportar.

Funciona sin conexión. Lo único que sale a la red es el enlace al repositorio
de la barra de estado, y solo si lo pulsas.

---

## Empezar

Al abrir la aplicación hay un lienzo en blanco de tipo arquitectura. Hay tres
maneras de arrancar:

| Cómo | Para qué |
| --- | --- |
| **Ejemplos** en la barra | Siete diagramas hechos, uno por tipo, para abrirlos y cambiarlos |
| **Importar** | Pegar código que ya tengas |
| Dibujar | Elegir el tipo en la barra y usar la paleta de formas |

El tipo de diagrama se elige en el desplegable **Diagrama** de la barra. No es
decorativo: decide qué formas ofrece la paleta, qué comprueba el validador y a
qué idiomas se puede generar código.

## La paleta de comandos

`Ctrl+K` abre una lista con todo lo que la aplicación sabe hacer. Se escribe
parte del nombre —sin preocuparse de tildes ni mayúsculas, y las palabras
valen en cualquier orden— y se pulsa Intro.

Es el sitio donde mirar cuando no se sabe dónde está algo. Los comandos que no
se pueden usar ahora mismo siguen en la lista, apagados, para que se vea que
existen y con qué atajo.

## Atajos

| Atajo | Acción |
| --- | --- |
| `Ctrl+K` | Paleta de comandos |
| `Ctrl+N` | Nuevo diagrama |
| `Ctrl+O` | Abrir |
| `Ctrl+S` · `Ctrl+Shift+S` | Guardar · Guardar como |
| `Ctrl+E` | Exportar |
| `Ctrl+Z` · `Ctrl+Shift+Z` | Deshacer · Rehacer |
| `V` · `H` | Puntero · Mano |
| `Supr` | Borrar la selección |

Dentro de un campo de texto mandan los atajos del campo: `Ctrl+Z` ahí deshace
la última letra, no la última operación del diagrama. La excepción es
`Ctrl+S`, que guarda siempre.

## Manejo del lienzo

| Gesto | Acción |
| --- | --- |
| Arrastrar con el puntero | Seleccionar por rectángulo |
| Arrastrar con la mano | Desplazar, también sobre un nodo |
| Barra espaciadora + arrastrar | Desplazar sin cambiar de herramienta |
| Botón central + arrastrar | Desplazar |
| Rueda | Desplazar · `Shift` horizontal · `Ctrl` zoom |
| Clic derecho | Menú contextual sobre nodo, relación o lienzo |
| `Ctrl` + clic | Añadir a la selección |

Arrastrar la punta de una relación la reengancha a otro nodo o a otro lado sin
perder su etiqueta ni su tipo.

## Los seis tipos

### Entidad-relación

Tablas con columnas, tipos, claves primarias y ajenas, unicidad y nulabilidad.
Cada fila de una tabla tiene su propio conector: arrastrar de una columna a
otra declara la relación, marca la clave ajena en el lado «muchos» y deja la
cardinalidad elegible entre las cuatro combinaciones de `1` y `*`.

Entra y sale en DBML (sintaxis de dbdiagram.io) y en `erDiagram` de Mermaid.

### Arquitectura

Cajas, contenedores anidados y relaciones con etiqueta, pensado para diagramas
de componentes y C4. Los contenedores se dibujan por detrás, así que una
flecha que acaba dentro de uno se sigue viendo entera.

Entra y sale en `flowchart` de Mermaid y en componentes de PlantUML.

### Clases

Cajas de clase con estereotipo, atributos y operaciones, cada miembro con su
visibilidad (`+`, `-`, `#`, `~`) y sus marcas de estático y abstracto. Las
relaciones llevan la semántica UML: triángulo hueco para herencia, rombos para
composición y agregación, trazo punteado para dependencia.

Entra y sale en `classDiagram` de Mermaid y en clases de PlantUML.

### Secuencia

Es el único tipo que no se dibuja colocando formas. Su geometría sale al cien
por cien del guion de mensajes, así que los participantes y los pasos se
añaden desde el panel del guion y el lienzo los coloca.

Admite mensajes síncronos, asíncronos y de retorno, creación y destrucción,
activaciones, notas y bloques `alt` / `opt` / `loop` / `par` / `critical` /
`break` / `group` con sus ramas.

Entra y sale en `sequenceDiagram` de Mermaid y en secuencias de PlantUML.

### Casos de uso

Actores, casos de uso dentro de la frontera del sistema, y las relaciones
entre ellos. El validador avisa si se usa una asociación donde toca una
dependencia («include» o «extend») o al revés.

Entra y sale en casos de uso de PlantUML.

### Estado y actividad

Estados redondeados, arranques y finales, bifurcaciones, barras de
sincronización y estados compuestos. En actividad, además, condiciones,
bucles y ramas paralelas.

Estado entra y sale en `stateDiagram-v2` de Mermaid y en estados de PlantUML;
actividad, en actividad de PlantUML.

## Importar código

El botón **Importar** abre un cuadro donde pegar el código. Se elige el
idioma; de qué dialecto se trata lo deduce la propia aplicación del contenido.

Lo que no se entiende **se dice, no se adivina**: cada problema sale con su
línea y su columna. Con errores no se importa nada —un diagrama a medias es
peor que ninguno, porque parece completo—; con avisos sí se importa, y los
avisos quedan en el panel de problemas.

Lo importado llega sin posiciones y lo coloca el auto-layout.

Qué entiende exactamente cada importador está en
[dialectos.md](dialectos.md).

## Generar código

El panel **Código** muestra el código del diagrama al día mientras se dibuja,
y permite copiarlo o guardarlo. Qué idiomas ofrece depende del tipo:

| Tipo | Idiomas |
| --- | --- |
| Entidad-relación | DBML, Mermaid |
| Arquitectura, clases, secuencia, estado | Mermaid, PlantUML |
| Casos de uso, actividad | PlantUML |

El camino de vuelta —escribir código y que cambie el diagrama— existe, pero
solo por el cuadro de importar. Rehacer el diagrama a cada tecla perdería las
posiciones ajustadas a mano.

## El panel de problemas

Los contadores de la barra de estado abren el panel. Lista dos cosas
distintas:

- **Del diagrama**: lo que el validador encuentra ahora mismo. Se rehace solo
  con cada cambio, y pulsar una línea selecciona lo que la provoca.
- **De la importación**: lo que se perdió al leer el código. No cambia con lo
  que se dibuje después y dura hasta que se abra otro documento.

La diferencia entre error y aviso es deliberada:

- **Error** solo si el diagrama se contradice: dos arranques en una máquina de
  estados, una herencia circular, un mensaje a un participante que no existe.
- **Aviso** para lo que se dibuja bien pero se notaría al exportar o al leerlo:
  una tabla vacía, un caso de uso al que no llega nadie, un paso sin salida.
- **Nada** para lo que es decisión de quien dibuja. Un diagrama a medias es un
  diagrama a medias, no un error.

## Exportar

`Ctrl+E` abre el cuadro de exportación: SVG, PNG (de 1× a 4×) y PDF, con
margen configurable y fondo opcionalmente transparente. También copia el SVG
al portapapeles.

Las tres salidas vienen del mismo generador, así que no pueden diferir entre
sí ni de lo que se ve en pantalla.

## Guardar y recuperar

Los diagramas se guardan en `.dgm`, que es JSON con versión de esquema.

Cada diez segundos se escribe una copia de recuperación en el directorio de
datos de la aplicación. **Nunca escribe sobre tu archivo.** Si la aplicación
se cerró sin guardar, al arrancar aparece un aviso flotante ofreciendo
recuperar; hay que pulsarlo, no se hace solo. Cerrar el aviso no borra nada:
para eso está «Descartar».

## Qué no hace

Dicho por delante, para no buscarlo:

- **No hay líneas guía de alineación** al arrastrar.
- **La cabecera de un participante de secuencia se dibuja siempre como caja**,
  nunca como monigote.
- **Fuera de un entidad-relación no se conectan columnas concretas**: las
  relaciones van de nodo a nodo.
- **No hay colaboración ni nube.** Es una aplicación de escritorio y un
  archivo.
