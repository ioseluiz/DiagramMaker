Aplicación de escritorio para diagramas de ingeniería de software: entidad-relación, arquitectura, clases, secuencia, casos de uso, actividad y estado. Se dibuja o se pega código PlantUML, Mermaid o DBML, y en los dos casos sale un diagrama editable que exporta a SVG, PNG y PDF. Funciona sin conexión.

## Qué descargar

| Archivo | Para |
| --- | --- |
| `*_x64_en-US.msi` | Windows, instalación normal |
| `*_x64-setup.exe` | Windows, instalador NSIS |
| `*_aarch64.dmg` | Mac con Apple Silicon (M1 en adelante) |
| `*_x64.dmg` | Mac con Intel |

## Antes de abrirla

**Los paquetes no van firmados.** No hay certificado de firma de código detrás de este proyecto, así que los dos sistemas van a avisar la primera vez.

En **Windows**, SmartScreen muestra «Windows protegió su PC». Hay que pulsar «Más información» y luego «Ejecutar de todas formas».

En **macOS** el aviso es más terminante: al abrir el `.app` dirá que está dañado o que no se puede comprobar el desarrollador. No está dañado; es que viene sin firmar y sin notarizar. Se abre así:

1. Arrastra la aplicación a `Aplicaciones` desde el `.dmg`.
2. Pulsa con el botón derecho sobre ella y elige **Abrir**, y confirma en el aviso. Con doble clic normal no deja; por el menú contextual, sí.

Si aun así se niega, quita la marca de cuarentena desde el Terminal:

```sh
xattr -dr com.apple.quarantine /Applications/DiagramMaker.app
```

## Documentación

- [Guía de uso](https://github.com/ioseluiz/DiagramMaker/blob/main/docs/guia-de-uso.md)
- [Qué entiende cada idioma](https://github.com/ioseluiz/DiagramMaker/blob/main/docs/dialectos.md)
