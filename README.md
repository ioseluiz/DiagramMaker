# DiagramMaker

Aplicación de escritorio para producir los diagramas que se usan a diario en
documentación técnica: entidad-relación, arquitectura/componentes, secuencia,
clases, casos de uso y actividad/estado.

Cubre las dos formas de trabajar con un solo modelo de datos:

- **Dibujar** de forma interactiva en un lienzo.
- **Importar** código PlantUML, Mermaid o DBML (sintaxis de dbdiagram.io) y
  obtener un diagrama nativo **editable**, no una imagen fija.

Exporta a SVG, PNG y PDF, y puede volver a generar código desde cualquier
diagrama. Funciona sin conexión a red.

Construido con Rust + Tauri 2 y React.

## Estado

**F3 — entidad-relación.** Pegas código DBML de dbdiagram.io y sale un
diagrama de tablas editable, colocado automáticamente. Y desde cualquier
diagrama ER puedes volver a generar el DBML. Lo siguiente es arquitectura/C4
con importador de Mermaid (F4).

| Fase | Alcance                                              |
| ---- | ---------------------------------------------------- |
| F0   | Andamiaje, CI, instaladores ✅                       |
| F1   | IR del documento, lienzo, undo/redo, persistencia ✅ |
| F2   | Exportación SVG / PNG / PDF ✅                       |
| F3   | Entidad-relación + importador DBML ✅                |
| F4   | Arquitectura / C4 + `flowchart` de Mermaid           |
| F5   | Secuencia (renderizador de carriles propio)          |
| F6   | Clases, casos de uso, actividad, estado              |
| F7   | Pulido, plantillas, instalador firmado               |

### Lo que ya funciona

- Ocho formas (rectángulo, redondeado, elipse, rombo, cilindro, nota, paquete,
  actor) dibujadas en SVG, con anidamiento dentro de paquetes.
- Ocho tipos de relación con su semántica UML: triángulo hueco para herencia,
  rombos para composición y agregación, trazo punteado para dependencia.
- El borde por el que enganchas una relación se respeta y se guarda. Arrastrar
  la punta de una arista la reengancha a otro nodo o a otro lado sin perder su
  etiqueta ni su tipo. Las relaciones importadas, que no traen lado elegido,
  lo calculan de la posición de las cajas.
- Deshacer y rehacer sobre parches de immer. Un arrastre completo colapsa en
  un solo paso, no en cien.
- Validación estructural continua: ids duplicados, aristas colgando,
  anidamiento circular.
- Autoguardado de recuperación cada 10 s en el directorio de datos de la
  aplicación. **Nunca escribe sobre el `.dgm` del usuario**: al arrancar
  ofrece recuperar en un aviso flotante, no lo hace solo. Cerrar el aviso no
  borra nada; para eso está «Descartar».
- Exportación a SVG, PNG (1× a 4×) y PDF, con margen configurable y fondo
  opcionalmente transparente. También copia el SVG al portapapeles.
- **Diagramas entidad-relación**: tablas con columnas, tipos, claves primarias
  y ajenas, unicidad y nulabilidad, agrupadas en contenedores. Se crean desde
  la paleta y se editan en el inspector: añadir, renombrar, reordenar y borrar
  columnas, cambiar su tipo y sus restricciones.
- **Agrupar, colorear y anotar**: encerrar tablas en un contenedor
  (seleccionarlas y «Agrupar», o arrastrarlas dentro), pintarlas con una
  paleta de tintes que funcionan en tema claro y oscuro, y escribir texto
  suelto sobre el lienzo.
- **Claves ajenas columna a columna**: cada fila de una tabla tiene su propio
  conector. Arrastrar de una columna a otra declara la relación, marca la FK
  en el lado «muchos» y deja la cardinalidad elegible entre las cuatro de
  DBML. Las cardinalidades se ven en el lienzo, no solo al exportar.
- **Importador de DBML** (sintaxis de dbdiagram.io): `Table` con sus ajustes,
  `Ref` sueltas y en línea, `Enum`, `TableGroup`, `Project` y notas. Lo que no
  entiende lo reporta con línea y columna en vez de adivinarlo, y sigue
  leyendo el resto. El resultado se coloca solo con ELK.
- **Generador de DBML**: desde cualquier diagrama ER sale el código, y vuelve
  a entrar produciendo el mismo diagrama (hay pruebas de ida y vuelta). El
  panel «Código» lo muestra al día mientras dibujas, y permite copiarlo o
  guardarlo como `.dbml`.

### Atajos

| Atajo                  | Acción             |
| ---------------------- | ------------------ |
| `Ctrl+N`               | Nuevo              |
| `Ctrl+O`               | Abrir              |
| `Ctrl+S` / `Ctrl+Shift+S` | Guardar / Guardar como |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Deshacer / Rehacer |
| `Supr`                 | Borrar la selección |

La paleta de formas depende del tipo de diagrama, que se elige en la barra de
herramientas: en un entidad-relación ofrece tablas, grupos y notas; en el
resto, el juego general de ocho formas.

### Manejo del lienzo

| Gesto | Acción |
| ----- | ------ |
| `V` / `H` | Alternar entre puntero y mano |
| Arrastrar con el puntero | Seleccionar por rectángulo |
| Arrastrar con la mano | Desplazar, también sobre un nodo |
| Barra espaciadora + arrastrar | Desplazar sin cambiar de herramienta |
| Botón central + arrastrar | Desplazar |
| Rueda | Desplazar · `Shift` horizontal · `Ctrl` zoom |
| Clic derecho | Menú contextual sobre nodo, relación o lienzo |
| `Ctrl` + clic | Añadir a la selección |

## Requisitos

| Herramienta      | Versión verificada |
| ---------------- | ------------------ |
| Rust             | 1.98.1             |
| Node.js          | 24.14              |
| MSVC Build Tools | 2026               |
| WebView2 Runtime | 155                |

## Puesta en marcha

```sh
npm install
npm run tauri dev
```

Si clonas el repositorio dentro de una carpeta sincronizada en la nube
(OneDrive, Dropbox, Drive), copia `.cargo/config.example.toml` a
`.cargo/config.toml` y ajusta `target-dir` a una ruta local. Dejar que `target/`
se sincronice degrada el build severamente y puede corromperlo.

## Comandos

| Comando                                           | Qué hace                                                      |
| ------------------------------------------------- | ------------------------------------------------------------- |
| `npm run tauri dev`                               | App en modo desarrollo con recarga en caliente                 |
| `npm run tauri build`                             | Instaladores MSI y NSIS en `src-tauri/target/release/bundle/`  |
| `npm test`                                        | Pruebas del frontend (Vitest)                                  |
| `npm run lint`                                    | ESLint, cero advertencias toleradas                            |
| `npm run format`                                  | Prettier sobre `src/`                                          |
| `npm run bindings`                                | Regenera los tipos TypeScript desde Rust (ts-rs)               |
| `npm run icon`                                    | Regenera los iconos desde `src-tauri/assets/icon.svg`          |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Pruebas del backend                                            |

## Arquitectura

```
Pegar código ──▶ parse::{dbml, mermaid, plantuml} ──┐
                                                     ▼
Dibujar ──▶ store (undo/redo) ──▶ geometría ──▶ model::Document (IR)
                 │                                   │
                 ├─▶ GraphCanvas (React Flow)        ├─▶ emit:: código
                 ├─▶ LaneCanvas  (SVG propio)        └─▶ render::svg ──▶ PNG (resvg)
                 └─▶ layout worker (elkjs)                          └──▶ PDF (svg2pdf)
```

Dos invariantes que sostienen el diseño:

1. **El esquema vive en Rust.** `src-tauri/src/model/` es la fuente única de
   verdad; los tipos TypeScript de `src/model/generated/` los genera `ts-rs` y
   **no se editan a mano**. CI falla si quedan desincronizados.
2. **Hay un solo generador de SVG, en Rust.** PNG (resvg) y PDF (svg2pdf)
   derivan de su salida, así que las tres exportaciones no pueden diferir
   entre sí.

   Rust no recalcula lo que solo el navegador sabe. Dos cosas llegan medidas
   desde el lienzo dentro de `RenderInput`:

   - **Las líneas de texto y su ancho.** `src/canvas/text.ts` parte y mide el
     texto; el lienzo pinta esas líneas y la exportación manda esas mismas.
     Rust solo las coloca. Es lo que evita que el PNG rompa el texto por otro
     sitio que la pantalla.
   - **El trazado de cada arista**, que calcula React Flow con su propio
     ruteo. Reimplementarlo en Rust daría curvas parecidas, no iguales.

   Lo que sí es matemática pura —el contorno de cada forma— vive en Rust
   (`model/shape_geometry.rs`). TypeScript lo reimplementa por rendimiento, y
   un fichero dorado compartido (`tests/golden/shape-primitives.json`) hace
   fallar una prueba en cuanto las dos versiones divergen.

### Estructura

```
src-tauri/src/
  commands.rs     superficie IPC (#[tauri::command])
  error.rs        AppError -> { kind, message } hacia el frontend
  model/          el IR del documento (F1)
  parse/dbml/     importador de DBML: lexer + parser con diagnósticos
  emit/dbml.rs    generador de DBML (ida y vuelta probada)
  render/         svg.rs (único generador), raster.rs (PNG), pdf.rs
  io/             .dgm versionado, recientes y copia de recuperación

src/
  model/          api.ts (único punto de invoke) + generated/
  store/          history.ts (undo/redo), operations.ts, documentStore.ts
  canvas/text.ts  medición y partido del texto (lo usan lienzo y exportación)
  canvas/graph/   adapter.ts (IR <-> React Flow), ShapeBody, ShapeNode
  canvas/export/  recolección de la geometría medida para exportar
  canvas/lane/    renderizador de secuencia (F5)
  layout/         auto-layout con elkjs
  theme/          tokens de color y preferencia de tema
  app/            shell de la ventana
```

## Licencias de terceros

Casi todo el stack es MIT o Apache-2.0. La excepción a tener presente:

- **`elkjs`** (motor de auto-layout) es **EPL-2.0 / GPL-3.0-or-later**. Es
  copyleft débil a nivel de archivo y se usa como dependencia sin modificar,
  que es su uso previsto. La alternativa MIT es `@dagrejs/dagre`, a costa de
  peor calidad en anidamiento y puertos.
