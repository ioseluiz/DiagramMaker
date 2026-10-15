/**
 * Marcadores SVG de los extremos de arista.
 *
 * React Flow solo trae flechas simples, y la semantica UML necesita triangulo
 * hueco (herencia) y rombos (composicion y agregacion). Se declaran una sola
 * vez en un `<svg>` de tamano cero y las aristas los referencian por `url(#id)`.
 *
 * `context-stroke` hace que el marcador tome el color de su propia arista, de
 * modo que un cambio de tema o de estilo no necesita regenerar los marcadores.
 */
export function EdgeMarkers() {
  return (
    <svg
      aria-hidden
      focusable="false"
      style={{ position: "absolute", width: 0, height: 0, overflow: "hidden" }}
    >
      <defs>
        <marker
          id="dm-arrow-open"
          viewBox="0 0 12 12"
          markerWidth="12"
          markerHeight="12"
          refX="11"
          refY="6"
          orient="auto-start-reverse"
          markerUnits="userSpaceOnUse"
        >
          <path
            d="M1,1 L11,6 L1,11"
            fill="none"
            stroke="context-stroke"
            strokeWidth="1.5"
          />
        </marker>

        <marker
          id="dm-arrow-filled"
          viewBox="0 0 12 12"
          markerWidth="11"
          markerHeight="11"
          refX="10"
          refY="6"
          orient="auto-start-reverse"
          markerUnits="userSpaceOnUse"
        >
          <path d="M1,1.5 L11,6 L1,10.5 z" fill="context-stroke" stroke="none" />
        </marker>

        <marker
          id="dm-triangle-hollow"
          viewBox="0 0 14 14"
          markerWidth="14"
          markerHeight="14"
          refX="13"
          refY="7"
          orient="auto-start-reverse"
          markerUnits="userSpaceOnUse"
        >
          <path
            d="M1,1 L13,7 L1,13 z"
            fill="var(--surface-0)"
            stroke="context-stroke"
            strokeWidth="1.5"
          />
        </marker>

        <marker
          id="dm-diamond-filled"
          viewBox="0 0 18 12"
          markerWidth="18"
          markerHeight="12"
          refX="1"
          refY="6"
          orient="auto-start-reverse"
          markerUnits="userSpaceOnUse"
        >
          <path d="M1,6 L9,1.5 L17,6 L9,10.5 z" fill="context-stroke" stroke="none" />
        </marker>

        <marker
          id="dm-diamond-hollow"
          viewBox="0 0 18 12"
          markerWidth="18"
          markerHeight="12"
          refX="1"
          refY="6"
          orient="auto-start-reverse"
          markerUnits="userSpaceOnUse"
        >
          <path
            d="M1,6 L9,1.5 L17,6 L9,10.5 z"
            fill="var(--surface-0)"
            stroke="context-stroke"
            strokeWidth="1.5"
          />
        </marker>
      </defs>
    </svg>
  );
}
