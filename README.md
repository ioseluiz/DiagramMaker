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

**F1 — documento y lienzo.** Ya se dibuja: crear formas, moverlas,
redimensionarlas, conectarlas, deshacer y rehacer, guardar y abrir `.dgm`, con
autoguardado de recuperación y archivos recientes. Falta la exportación (F2).

| Fase | Alcance                                              |
| ---- | ---------------------------------------------------- |
| F0   | Andamiaje, CI, instaladores ✅                       |
| F1   | IR del documento, lienzo, undo/redo, persistencia ✅ |
| F2   | Exportación SVG / PNG / PDF                          |
| F3   | Entidad-relación + importador DBML                   |
| F4   | Arquitectura / C4 + `flowchart` de Mermaid           |
| F5   | Secuencia (renderizador de carriles propio)          |
| F6   | Clases, casos de uso, actividad, estado              |
| F7   | Pulido, plantillas, instalador firmado               |

### Lo que ya funciona

- Ocho formas (rectángulo, redondeado, elipse, rombo, cilindro, nota, paquete,
  actor) dibujadas en SVG, con anidamiento dentro de paquetes.
- Ocho tipos de relación con su semántica UML: triángulo hueco para herencia,
  rombos para composición y agregación, trazo punteado para dependencia.
- Deshacer y rehacer sobre parches de immer. Un arrastre completo colapsa en
  un solo paso, no en cien.
- Validación estructural continua: ids duplicados, aristas colgando,
  anidamiento circular.
- Autoguardado de recuperación cada 10 s en el directorio de datos de la
  aplicación. **Nunca escribe sobre el `.dgm` del usuario**: al arrancar
  ofrece recuperar, no lo hace solo.

### Atajos

| Atajo                  | Acción             |
| ---------------------- | ------------------ |
| `Ctrl+N`               | Nuevo              |
| `Ctrl+O`               | Abrir              |
| `Ctrl+S` / `Ctrl+Shift+S` | Guardar / Guardar como |
| `Ctrl+Z` / `Ctrl+Shift+Z` | Deshacer / Rehacer |
| `Supr`                 | Borrar la selección |

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
2. **El SVG se genera en Rust**, a partir del documento más la geometría
   resuelta — nunca se extrae del DOM. PNG y PDF derivan de ese mismo SVG, así
   que las tres salidas coinciden por construcción.

### Estructura

```
src-tauri/src/
  commands.rs     superficie IPC (#[tauri::command])
  error.rs        AppError -> { kind, message } hacia el frontend
  model/          el IR del documento (F1)
  parse/          importadores DBML / Mermaid / PlantUML (F3-F6)
  emit/           generadores de código (F3-F6)
  render/         svg.rs, raster.rs, pdf.rs (F2)
  io/             .dgm versionado, recientes y copia de recuperación

src/
  model/          api.ts (único punto de invoke) + generated/
  store/          history.ts (undo/redo), operations.ts, documentStore.ts
  canvas/graph/   adapter.ts (IR <-> React Flow), ShapeBody, ShapeNode
  canvas/lane/    renderizador de secuencia (F5)
  layout/         worker de elkjs (F3)
  theme/          tokens de color y preferencia de tema
  app/            shell de la ventana
```

## Licencias de terceros

Casi todo el stack es MIT o Apache-2.0. La excepción a tener presente:

- **`elkjs`** (motor de auto-layout) es **EPL-2.0 / GPL-3.0-or-later**. Es
  copyleft débil a nivel de archivo y se usa como dependencia sin modificar,
  que es su uso previsto. La alternativa MIT es `@dagrejs/dagre`, a costa de
  peor calidad en anidamiento y puertos.
