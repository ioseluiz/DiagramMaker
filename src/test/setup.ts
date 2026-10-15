import "@testing-library/jest-dom/vitest";

// jsdom no implementa canvas y avisa por consola en cada llamada. Devolver
// `null` es exactamente lo que ocurre cuando no hay contexto 2D, que es el
// caso que `@/canvas/text` ya contempla con su estimacion de respaldo.
HTMLCanvasElement.prototype.getContext = () => null;
