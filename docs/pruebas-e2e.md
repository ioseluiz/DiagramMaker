# Pruebas de extremo a extremo

El recorrido completo de la aplicación está cubierto por dos pruebas que se
reparten el viaje por la mitad. Ninguna necesita un navegador de verdad, y por
eso corren en CI con todo lo demás.

Falta la mitad que sí lo necesitaría. Este documento dice qué cubre cada cosa,
qué queda fuera y cómo montarlo el día que haya una máquina donde se pueda.

---

## Lo que hay

### `src-tauri/tests/recorrido.rs` — el viaje por el backend

Hace lo que hace un usuario, en el mismo orden y por las mismas funciones que
llama la aplicación:

```
pegar código → documento → validar → SVG → PNG y PDF → guardar .dgm
→ cerrar → reabrir → el mismo documento
```

No busca que cada pieza funcione —para eso están las pruebas de cada una—
sino que **encajen**: que lo que sale de una entre en la siguiente sin que
nadie tenga que retocarlo por el camino. Hay una segunda prueba que repite el
viaje, más corto, entrando por cada uno de los tres idiomas.

Lo que no pasa por ahí es el navegador. Medir el texto y rutear las aristas es
cosa suya, así que la prueba lo imita a lo bruto.

### `src/app/App.test.tsx` — el viaje por la interfaz

Monta `App` entera con su store de verdad y recorre: abrir un ejemplo, ver el
diagrama dibujado, mirar su código, provocar un problema y pulsarlo para que
seleccione al culpable, usar la paleta de comandos, deshacer, y comprobar que
se escribe la copia de recuperación.

Lo simulado es el puente con Rust y el auto-layout. De esa mitad se encarga
`recorrido.rs`.

El diagrama que abre es una **secuencia** a propósito: su lienzo es SVG propio
y se monta en jsdom. El lienzo de grafo usa React Flow, cuyo `ZoomPane` mide y
transforma con el navegador y revienta sin él, así que ahí se sustituye por
una marca. La prueba comprueba que el armazón elige el lienzo que toca según
el tipo de diagrama, pero no lo que pasa dentro del de grafo.

## Lo que falta

Nada de lo anterior toca el **WebView real**. Queda sin cubrir:

- Arrastrar un nodo con el ratón y que se quede donde se soltó.
- El zoom, el desplazamiento y la selección por rectángulo.
- Enganchar una relación arrastrando de un borde a otro.
- Que la ventana nativa abra, se redimensione y cierre.
- Los diálogos de archivo del sistema.

## Cómo montarlo

Hace falta, además de lo que ya trae el repositorio:

| Pieza | De dónde | Hace falta admin |
| --- | --- | --- |
| `tauri-driver` | `cargo install tauri-driver` | No: va a `~/.cargo/bin` |
| `msedgedriver.exe` | `msedgedriver.microsoft.com`, **la misma versión que WebView2** | Depende de dónde se deje |
| WebdriverIO | `npm i -D @wdio/cli @wdio/local-runner @wdio/mocha-framework @wdio/spec-reporter` | No |

La versión de WebView2 se consulta así:

```powershell
Get-ChildItem "C:\Program Files (x86)\Microsoft\EdgeWebView\Application" -Directory |
  Where-Object { $_.Name -match '^\d+\.' } | Select-Object -Last 1 -ExpandProperty Name
```

El driver tiene que ser exactamente de esa versión. Es el motivo más frecuente
de que esto no arranque.

El recorrido que deberían cubrir, según el plan del proyecto:

> abrir la app → pegar DBML → ver el diagrama → mover un nodo → exportar PNG →
> reabrir el archivo

Dos cosas que conviene decidir antes de escribirlas:

- **Necesitan la aplicación construida** (`npm run tauri build`), no el modo
  de desarrollo. Son varios minutos por ejecución.
- **No deberían entrar en CI sin más.** Necesitan sesión gráfica y son
  propensas a fallar por razones que no son del código. Si entran, que sea
  como trabajo aparte que no bloquee, y no mezcladas con el resto.

## Por qué no están

En la máquina donde se desarrolla esto no se pueden instalar ejecutables. Las
dos pruebas que sí hay se escribieron para cubrir todo lo que se podía sin
eso, que resultó ser casi todo salvo el lienzo de grafo y la ventana nativa.
