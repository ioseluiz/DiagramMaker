Aplicación de escritorio para diagramas de ingeniería de software: entidad-relación, arquitectura, clases, secuencia, casos de uso, actividad y estado. Se dibuja o se pega código PlantUML, Mermaid o DBML, y en los dos casos sale un diagrama editable que exporta a SVG, PNG y PDF. Funciona sin conexión.

## Qué cambia en la 0.1.1

Todo son correcciones. La 0.1.0 arrastraba fallos que se notaban en cuanto se pegaba código real o se exportaba a color.

**La aplicación se cerraba sola.** Al desplazar el lienzo o mover un nodo en un diagrama grande, la ventana desaparecía sin aviso. React Flow medía el nodo en el navegador, guardábamos esa medida, eso rehacía la lista de nodos y él la volvía a medir; como la medida sale de dividir píxeles entre el zoom, con un desplazamiento fraccionario nunca convergía. Ahora las dimensiones las declara el documento y no se realimentan.

**Se cerraba también con acentos.** El lector de `flowchart` recorría la línea byte a byte, así que una `ó` salía como `Ã³` y, si caía fuera de unos corchetes, el corte siguiente entraba en pánico. Con `panic = "abort"` eso se llevaba la aplicación entera. Corregido en los doce caminos de entrada, no solo en el que fallaba.

**El `&` de Mermaid se leía mal.** `a & b --> c` declara dos nodos a cada lado; antes partía la línea por donde no era. Y `[Ventas & Marketing]` ya no se parte: dentro de una etiqueta el `&` es texto.

**Los colores salían negros al exportar.** Los rellenos se escribían con la sintaxis CSS moderna `rgb(37 99 235 / 0.16)`, que el navegador entiende y `resvg` no — y lo que no reconoce lo pinta negro, sin avisar. Se veía bien en pantalla y salía negro en el PNG y el PDF. Los documentos ya guardados también quedan arreglados: la traducción ocurre al exportar.

**Los paquetes con muchos elementos salían comprimidos.** Los contenedores ya ajustan su tamaño al contenido en vez de quedarse con una medida fija.

**El clic no seleccionaba nada.** Ni relaciones ni cajas. Con una caja se podía disimular encerrándola en un rectángulo de selección, que iba por otro camino; con una línea no, porque el rectángulo solo coge nodos.

Y dos cosas que no estaban:

- **Tamaño del texto de un nodo**, con el interlineado siguiendo al cuerpo de la letra y la caja ajustándose al contenido.
- **Grosor y trazado de una relación**, que el modelo ya guardaba pero no se podían cambiar desde la interfaz.

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
