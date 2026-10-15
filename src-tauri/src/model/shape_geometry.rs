use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::node::NodeShape;

/// Contorno de una forma, descompuesto en primitivas SVG.
///
/// Es la **unica** definicion de como se dibuja cada forma. El lienzo
/// (`src/canvas/graph/ShapeBody.tsx`) y el exportador de SVG consumen esta
/// misma lista, de modo que lo que se ve en pantalla y lo que sale exportado
/// no pueden divergir por accidente. `tests/golden/shape-primitives.json`
/// vigila que la copia de TypeScript siga coincidiendo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum Primitive {
    #[serde(rename_all = "camelCase")]
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional)]
        radius: Option<f64>,
    },
    #[serde(rename_all = "camelCase")]
    Ellipse { cx: f64, cy: f64, rx: f64, ry: f64 },
    #[serde(rename_all = "camelCase")]
    Circle { cx: f64, cy: f64, r: f64 },
    /// Trazo cerrado y relleno.
    #[serde(rename_all = "camelCase")]
    Path { d: String },
    /// Linea interior sin relleno: la tapa del cilindro, el doblez de la nota,
    /// los miembros del actor.
    #[serde(rename_all = "camelCase")]
    Line { d: String },
}

impl Primitive {
    /// `true` si la primitiva se rellena. Las lineas interiores no.
    pub fn is_filled(&self) -> bool {
        !matches!(self, Primitive::Line { .. })
    }
}

const CORNER_RADIUS: f64 = 10.0;
/// Alto de la solapa del paquete.
const TAB_HEIGHT: f64 = 22.0;
/// Alto maximo de la tapa eliptica del cilindro.
const CYLINDER_LIP: f64 = 12.0;

/// El trazo se dibuja centrado en el borde, asi que medio grosor se sale de la
/// caja. Encogiendo este margen por lado, el contorno queda dentro.
const INSET: f64 = 0.75;

/// Redondea a 4 decimales. Sin esto, dos implementaciones del mismo calculo en
/// lenguajes distintos divergen en el ultimo bit y el fichero dorado deja de
/// coincidir por ruido, no por un cambio real.
fn r(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

/// Contorno de `shape` dentro de una caja de `w` x `h`, en coordenadas locales
/// (origen en la esquina superior izquierda de la caja).
pub fn primitives(shape: NodeShape, w: f64, h: f64) -> Vec<Primitive> {
    let i = INSET;

    match shape {
        NodeShape::Rectangle => vec![Primitive::Rect {
            x: r(i),
            y: r(i),
            width: r(w - i * 2.0),
            height: r(h - i * 2.0),
            radius: None,
        }],

        NodeShape::RoundedRect => vec![Primitive::Rect {
            x: r(i),
            y: r(i),
            width: r(w - i * 2.0),
            height: r(h - i * 2.0),
            radius: Some(CORNER_RADIUS),
        }],

        NodeShape::Ellipse => vec![Primitive::Ellipse {
            cx: r(w / 2.0),
            cy: r(h / 2.0),
            rx: r(w / 2.0 - i),
            ry: r(h / 2.0 - i),
        }],

        NodeShape::Diamond => vec![Primitive::Path {
            d: format!(
                "M {} {} L {} {} L {} {} L {} {} Z",
                r(w / 2.0),
                r(i),
                r(w - i),
                r(h / 2.0),
                r(w / 2.0),
                r(h - i),
                r(i),
                r(h / 2.0)
            ),
        }],

        NodeShape::Cylinder => {
            let ry = (CYLINDER_LIP).min(h / 4.0);
            let rx = w / 2.0 - i;
            vec![
                Primitive::Path {
                    d: format!(
                        "M {} {} A {} {} 0 0 1 {} {} L {} {} A {} {} 0 0 1 {} {} Z",
                        r(i),
                        r(ry + i),
                        r(rx),
                        r(ry),
                        r(w - i),
                        r(ry + i),
                        r(w - i),
                        r(h - ry - i),
                        r(rx),
                        r(ry),
                        r(i),
                        r(h - ry - i)
                    ),
                },
                Primitive::Line {
                    d: format!(
                        "M {} {} A {} {} 0 0 0 {} {}",
                        r(i),
                        r(ry + i),
                        r(rx),
                        r(ry),
                        r(w - i),
                        r(ry + i)
                    ),
                },
            ]
        }

        NodeShape::Note => {
            let fold = 18.0_f64.min(w / 4.0).min(h / 3.0);
            vec![
                Primitive::Path {
                    d: format!(
                        "M {} {} L {} {} L {} {} L {} {} L {} {} Z",
                        r(i),
                        r(i),
                        r(w - fold - i),
                        r(i),
                        r(w - i),
                        r(fold + i),
                        r(w - i),
                        r(h - i),
                        r(i),
                        r(h - i)
                    ),
                },
                Primitive::Line {
                    d: format!(
                        "M {} {} L {} {} L {} {}",
                        r(w - fold - i),
                        r(i),
                        r(w - fold - i),
                        r(fold + i),
                        r(w - i),
                        r(fold + i)
                    ),
                },
            ]
        }

        NodeShape::Package => {
            let tab_width = (w * 0.45).min(120.0);
            vec![
                Primitive::Path {
                    d: format!(
                        "M {} {} L {} {} L {} {} L {} {}",
                        r(i),
                        r(TAB_HEIGHT + i),
                        r(i),
                        r(i),
                        r(tab_width),
                        r(i),
                        r(tab_width),
                        r(TAB_HEIGHT + i)
                    ),
                },
                Primitive::Rect {
                    x: r(i),
                    y: r(TAB_HEIGHT + i),
                    width: r(w - i * 2.0),
                    height: r(h - TAB_HEIGHT - i * 2.0),
                    radius: None,
                },
            ]
        }

        NodeShape::Actor => {
            let cx = w / 2.0;
            let head_r = (w / 3.0).min(h / 6.0);
            let head_cy = head_r + i;
            let neck = head_cy + head_r;
            let hip = neck + (h - neck - i) * 0.5;
            let arms = neck + (hip - neck) * 0.3;

            vec![
                Primitive::Circle {
                    cx: r(cx),
                    cy: r(head_cy),
                    r: r(head_r),
                },
                Primitive::Line {
                    d: format!(
                        "M {} {} L {} {} M {} {} L {} {} M {} {} L {} {} M {} {} L {} {}",
                        r(cx),
                        r(neck),
                        r(cx),
                        r(hip),
                        r(i),
                        r(arms),
                        r(w - i),
                        r(arms),
                        r(cx),
                        r(hip),
                        r(i),
                        r(h - i),
                        r(cx),
                        r(hip),
                        r(w - i),
                        r(h - i)
                    ),
                },
            ]
        }
    }
}

/// Todas las formas, para recorrerlas en pruebas y en la paleta.
pub const ALL_SHAPES: [NodeShape; 8] = [
    NodeShape::Rectangle,
    NodeShape::RoundedRect,
    NodeShape::Ellipse,
    NodeShape::Diamond,
    NodeShape::Cylinder,
    NodeShape::Note,
    NodeShape::Package,
    NodeShape::Actor,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_rectangulo_cabe_dentro_de_su_caja() {
        let p = primitives(NodeShape::Rectangle, 100.0, 50.0);

        assert_eq!(
            p,
            vec![Primitive::Rect {
                x: 0.75,
                y: 0.75,
                width: 98.5,
                height: 48.5,
                radius: None,
            }]
        );
    }

    #[test]
    fn solo_el_rectangulo_redondeado_lleva_radio() {
        let plain = primitives(NodeShape::Rectangle, 100.0, 50.0);
        let rounded = primitives(NodeShape::RoundedRect, 100.0, 50.0);

        assert!(matches!(plain[0], Primitive::Rect { radius: None, .. }));
        assert!(matches!(
            rounded[0],
            Primitive::Rect {
                radius: Some(CORNER_RADIUS),
                ..
            }
        ));
    }

    #[test]
    fn el_cilindro_tiene_cuerpo_relleno_y_tapa_sin_rellenar() {
        let p = primitives(NodeShape::Cylinder, 120.0, 90.0);

        assert_eq!(p.len(), 2);
        assert!(p[0].is_filled());
        assert!(!p[1].is_filled());
    }

    #[test]
    fn la_tapa_del_cilindro_se_achata_en_cajas_bajas() {
        // Con h = 90 la tapa mide el maximo (12); con h = 24 queda en h/4 = 6.
        let alto = primitives(NodeShape::Cylinder, 120.0, 90.0);
        let bajo = primitives(NodeShape::Cylinder, 120.0, 24.0);

        let Primitive::Line { d: alto_d } = &alto[1] else {
            panic!("se esperaba una linea");
        };
        let Primitive::Line { d: bajo_d } = &bajo[1] else {
            panic!("se esperaba una linea");
        };

        assert!(alto_d.contains(" 12 "), "{alto_d}");
        assert!(bajo_d.contains(" 6 "), "{bajo_d}");
    }

    #[test]
    fn el_doblez_de_la_nota_no_se_come_la_caja() {
        // El doblez se limita a w/4 y h/3 para que una nota pequena siga
        // pareciendo una nota: con 20x20 sale 5, no los 18 del maximo.
        let p = primitives(NodeShape::Note, 20.0, 20.0);

        let Primitive::Path { d } = &p[0] else {
            panic!("se esperaba un trazo");
        };
        // 20 - 5 (doblez) - 0.75 (margen de trazo) = 14.25
        assert!(d.contains("L 14.25 0.75"), "{d}");
    }

    #[test]
    fn el_actor_es_cabeza_mas_miembros() {
        let p = primitives(NodeShape::Actor, 40.0, 80.0);

        assert!(matches!(p[0], Primitive::Circle { .. }));
        assert!(matches!(p[1], Primitive::Line { .. }));
    }

    #[test]
    fn ninguna_forma_se_queda_sin_primitivas() {
        for shape in ALL_SHAPES {
            assert!(
                !primitives(shape, 160.0, 60.0).is_empty(),
                "{shape:?} no genero primitivas"
            );
        }
    }

    #[test]
    fn todas_las_formas_tienen_al_menos_una_primitiva_rellena() {
        for shape in ALL_SHAPES {
            assert!(
                primitives(shape, 160.0, 60.0)
                    .iter()
                    .any(Primitive::is_filled),
                "{shape:?} no tiene cuerpo"
            );
        }
    }
}
