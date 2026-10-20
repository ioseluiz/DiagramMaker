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

## Descargas

Los paquetes de cada versión están en
[Releases](https://github.com/ioseluiz/DiagramMaker/releases):

| Archivo | Para |
| --- | --- |
| `*_x64_en-US.msi` | Windows, instalación normal |
| `*_x64-setup.exe` | Windows, instalador NSIS |
| `*_aarch64.dmg` | Mac con Apple Silicon |
| `*_x64.dmg` | Mac con Intel |

**No van firmados**: no hay certificado de firma de código detrás de este
proyecto. Windows mostrará el aviso de SmartScreen —«Más información» y
«Ejecutar de todas formas»— y macOS dirá que la aplicación está dañada o que
no puede comprobar al desarrollador. No lo está; se abre pulsando con el botón
derecho y eligiendo **Abrir**, o quitando la marca de cuarentena:

```sh
xattr -dr com.apple.quarantine /Applications/DiagramMaker.app
```

Se publica empujando un tag `vX.Y.Z`: el flujo
[`release.yml`](.github/workflows/release.yml) construye las tres
plataformas y solo crea la release si salen las tres.

## Estado

**F7 — pulido.** Los seis tipos de diagrama entran y salen ya en todos los
idiomas del plan, con paleta de comandos, ejemplos de arranque, validación por
tipo y documentación. El instalador **no se firma**: sin certificado, Windows
mostrará el aviso de SmartScreen la primera vez.

| Fase | Alcance                                              |
| ---- | ---------------------------------------------------- |
| F0   | Andamiaje, CI, instaladores ✅                       |
| F1   | IR del documento, lienzo, undo/redo, persistencia ✅ |
| F2   | Exportación SVG / PNG / PDF ✅                       |
| F3   | Entidad-relación + importador DBML ✅                |
| F4   | Arquitectura / C4 + Mermaid y PlantUML ✅            |
| F5   | Secuencia (renderizador de carriles propio) ✅       |
| F6   | Clases, casos de uso, actividad, estado ✅           |
| F7   | Pulido, plantillas, documentación, pruebas — en curso |

### Documentación

- **[Guía de uso](docs/guia-de-uso.md)** — cómo se usa, atajos, qué hace cada
  panel y qué no hace.
- **[Qué entiende cada idioma](docs/dialectos.md)** — la frontera exacta de
  cada importador y generador, y qué se pierde por el camino.
- **[Pruebas de extremo a extremo](docs/pruebas-e2e.md)** — qué cubre cada
  una, qué queda fuera y cómo montar las que faltan.

### Lo que ya funciona

**Los seis tipos de diagrama**, cada uno con sus formas, sus reglas y sus
idiomas:

| Tipo | Entra y sale en |
| --- | --- |
| Entidad-relación | DBML, `erDiagram` de Mermaid |
| Arquitectura / C4 | `flowchart` de Mermaid, componentes de PlantUML |
| Clases | `classDiagram` de Mermaid, clases de PlantUML |
| Secuencia | `sequenceDiagram` de Mermaid, secuencias de PlantUML |
| Estado | `stateDiagram-v2` de Mermaid, estados de PlantUML |
| Casos de uso | Casos de uso de PlantUML |
| Actividad | Actividad de PlantUML |

Todos los generadores tienen pruebas de ida y vuelta: importar, generar y
volver a importar da el mismo diagrama.

**El lienzo**

- Dieciséis formas dibujadas en SVG y ocho tipos de relación con su semántica
  UML: triángulo hueco para herencia, rombos para composición y agregación,
  trazo punteado para dependencia.
- El borde por el que enganchas una relación se respeta y se guarda. Arrastrar
  la punta la reengancha a otro nodo o a otro lado sin perder su etiqueta ni
  su tipo.
- **Saltos en los cruces**: cuando dos relaciones se cruzan sin estar
  conectadas, la de encima dibuja un puente. Sin eso un cruce y una unión se
  ven idénticos.
- Los contenedores se dibujan por detrás de las relaciones, así que una flecha
  que acaba dentro de uno se ve entera, con su punta.
- Deshacer y rehacer sobre parches de immer. Un arrastre completo colapsa en
  un solo paso, no en cien.
- Agrupar, colorear con tintes que funcionan en los dos temas, y escribir
  texto suelto sobre el lienzo.
- El renderizador de carriles propio para secuencia, cuya geometría sale al
  cien por cien del guion de mensajes.

**Alrededor**

- **Paleta de comandos** (`Ctrl+K`) con todo lo que la aplicación sabe hacer,
  buscando sin tildes ni mayúsculas. Es el mismo registro del que salen los
  atajos, así que no pueden separarse.
- **Siete diagramas de arranque**, uno por tipo. Están escritos en código y
  entran por el importador, así que son además una prueba viva de los parsers.
- **Validación continua** en dos niveles: estructural para cualquier documento
  e idiomática por tipo —dos arranques en una máquina de estados, una herencia
  circular, un caso de uso al que no llega nadie—. El panel de problemas las
  lista y pulsar una selecciona lo que la provoca.
- **Autoguardado de recuperación** cada 10 s en el directorio de datos.
  **Nunca escribe sobre tu `.dgm`**: al arrancar ofrece recuperar en un aviso,
  no lo hace solo.
- Exportación a SVG, PNG (1× a 4×) y PDF, con margen configurable y fondo
  opcionalmente transparente. También copia el SVG al portapapeles.

### Atajos

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

El resto del manejo del lienzo está en la
[guía de uso](docs/guia-de-uso.md#manejo-del-lienzo).

La ventana admite hasta 960 × 600. Por debajo de lo que cabe en una fila, la
barra de herramientas pasa a dos filas en lugar de recortarse, y los paneles
laterales se limitan a un tercio del ancho para que siempre quede lienzo.

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
  model/          el IR del documento, su geometría y su validación
  parse/          importadores: dbml/, mermaid/, plantuml/, con diagnósticos
  emit/           generadores, uno por idioma y dialecto
  render/         svg.rs (único generador), raster.rs (PNG), pdf.rs, lane.rs
  templates.rs    los siete diagramas de arranque, escritos en código
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
