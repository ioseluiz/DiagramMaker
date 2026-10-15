import { beforeEach, describe, expect, it } from "vitest";

import type { Document } from "@/model/generated/Document";
import type { Node } from "@/model/generated/Node";
import type { NodeShape } from "@/model/generated/NodeShape";
import {
  DEFAULT_EXPORT_OPTIONS,
  __testing,
  collectRenderInput,
  resolveTheme,
} from "./collect";

function node(id: string, label: string, shape: NodeShape = "rectangle"): Node {
  return {
    id,
    label,
    shape,
    geometry: { x: 0, y: 0, width: 160, height: 60 },
    pinned: false,
    data: { type: "plain" },
  };
}

function doc(nodes: Node[] = [], edges: Document["edges"] = []): Document {
  return {
    schemaVersion: 1,
    id: "d1",
    kind: "architecture",
    meta: { title: "Prueba", created: 0, modified: 0 },
    nodes,
    edges,
    layout: "manual",
    direction: "right",
  };
}

describe("resolveTheme", () => {
  it("devuelve colores concretos, nunca variables CSS", () => {
    const theme = resolveTheme();

    for (const [key, value] of Object.entries(theme)) {
      expect(value, key).not.toMatch(/var\(--/);
      expect(value.length, key).toBeGreaterThan(0);
    }
  });
});

describe("etiquetas de nodo", () => {
  it("mide y parte la etiqueta de cada nodo", () => {
    const labels = __testing.nodeLabels(doc([node("a", "API")]));

    expect(labels.a).toBeDefined();
    expect(labels.a!.lines[0]!.text).toBe("API");
    expect(labels.a!.lines[0]!.width).toBeGreaterThan(0);
  });

  it("omite los nodos sin etiqueta", () => {
    const labels = __testing.nodeLabels(doc([node("a", "   ")]));

    expect(labels.a).toBeUndefined();
  });

  it("el actor rotula por debajo y el paquete en la solapa", () => {
    const labels = __testing.nodeLabels(
      doc([node("a", "Operador", "actor"), node("p", "Dominio", "package")]),
    );

    expect(labels.a!.placement).toBe("below");
    expect(labels.p!.placement).toBe("tab");
  });

  it("la solapa del paquete usa una fuente menor", () => {
    const labels = __testing.nodeLabels(
      doc([node("a", "Caja"), node("p", "Dominio", "package")]),
    );

    expect(labels.p!.fontSize).toBeLessThan(labels.a!.fontSize);
  });

  it("una etiqueta larga se parte en varias lineas", () => {
    const labels = __testing.nodeLabels(
      doc([node("a", "Servicio de autenticacion y autorizacion centralizado")]),
    );

    expect(labels.a!.lines.length).toBeGreaterThan(1);
  });
});

describe("etiquetas de arista", () => {
  const twoNodes = [node("a", "A"), node("b", "B")];

  it("mide la etiqueta central", () => {
    const labels = __testing.edgeLabels(
      doc(twoNodes, [
        {
          id: "e1",
          source: { node: "a" },
          target: { node: "b" },
          kind: "flow",
          routing: "orthogonal",
          label: "consulta",
        },
      ]),
    );

    expect(labels.e1!.lines[0]!.text).toBe("consulta");
  });

  it("omite las aristas sin etiqueta", () => {
    const labels = __testing.edgeLabels(
      doc(twoNodes, [
        {
          id: "e1",
          source: { node: "a" },
          target: { node: "b" },
          kind: "flow",
          routing: "orthogonal",
        },
      ]),
    );

    expect(labels.e1).toBeUndefined();
  });
});

describe("trazados de arista", () => {
  const edges: Document["edges"] = [
    {
      id: "e1",
      source: { node: "a" },
      target: { node: "b" },
      kind: "flow",
      routing: "orthogonal",
    },
  ];

  beforeEach(() => {
    document.body.innerHTML = "";
  });

  it("lee el `d` que dibujo React Flow", () => {
    document.body.innerHTML = `
      <svg>
        <g class="react-flow__edge" data-id="e1">
          <path class="react-flow__edge-path" d="M 0 0 L 100 50"></path>
        </g>
      </svg>`;

    const paths = __testing.edgePaths(doc([], edges));

    expect(paths.e1!.d).toBe("M 0 0 L 100 50");
  });

  it("omite la arista si no esta dibujada", () => {
    const paths = __testing.edgePaths(doc([], edges));

    expect(paths.e1).toBeUndefined();
  });

  it("no se confunde con la arista de al lado", () => {
    document.body.innerHTML = `
      <svg>
        <g class="react-flow__edge" data-id="otra">
          <path class="react-flow__edge-path" d="M 9 9 L 9 9"></path>
        </g>
        <g class="react-flow__edge" data-id="e1">
          <path class="react-flow__edge-path" d="M 0 0 L 100 50"></path>
        </g>
      </svg>`;

    expect(__testing.edgePaths(doc([], edges)).e1!.d).toBe("M 0 0 L 100 50");
  });

  it("un id con caracteres raros no rompe el selector", () => {
    const raros: Document["edges"] = [
      {
        id: 'e"1',
        source: { node: "a" },
        target: { node: "b" },
        kind: "flow",
        routing: "orthogonal",
      },
    ];

    expect(() => __testing.edgePaths(doc([], raros))).not.toThrow();
  });
});

describe("collectRenderInput", () => {
  it("junta documento, etiquetas, trazados, tema y opciones", () => {
    const d = doc([node("a", "API")]);

    const input = collectRenderInput(d);

    expect(input.document).toBe(d);
    expect(input.labels.a).toBeDefined();
    expect(input.theme.fontFamily.length).toBeGreaterThan(0);
    expect(input.options).toEqual(DEFAULT_EXPORT_OPTIONS);
  });

  it("respeta las opciones que se le pasan", () => {
    const input = collectRenderInput(doc(), {
      padding: 0,
      opaqueBackground: false,
      scale: 4,
    });

    expect(input.options).toEqual({ padding: 0, opaqueBackground: false, scale: 4 });
  });
});
