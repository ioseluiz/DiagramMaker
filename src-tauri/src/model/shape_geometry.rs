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

/// Alto de la cabecera de una tabla ER, donde va su nombre.
pub const TABLE_HEADER_HEIGHT: f64 = 28.0;
/// Alto de cada fila de columna.
pub const TABLE_ROW_HEIGHT: f64 = 22.0;
/// Margen horizontal del texto dentro de la tabla.
pub const TABLE_PADDING: f64 = 10.0;
/// Tamano del nombre de la tabla, en la cabecera.
pub const TABLE_HEADER_FONT_SIZE: f64 = 13.0;
/// Tamano del texto de cada columna.
pub const TABLE_ROW_FONT_SIZE: f64 = 12.0;

/// Alto que necesita una tabla con `rows` columnas.
pub fn table_height(rows: usize) -> f64 {
    TABLE_HEADER_HEIGHT + rows as f64 * TABLE_ROW_HEIGHT
}

/// Caja de la fila `index` dentro de una tabla de ancho `w`, en coordenadas
/// locales al nodo.
pub fn table_row_rect(index: usize, w: f64) -> super::geometry::Rect {
    super::geometry::Rect::new(
        0.0,
        TABLE_HEADER_HEIGHT + index as f64 * TABLE_ROW_HEIGHT,
        w,
        TABLE_ROW_HEIGHT,
    )
}

/// Contorno de una tabla ER: marco, separador de cabecera y una linea entre
/// filas.
///
/// Va aparte de `primitives` porque depende del numero de columnas, que no es
/// deducible del tamano de la caja.
pub fn table_primitives(rows: usize, w: f64, h: f64) -> Vec<Primitive> {
    let i = INSET;
    let mut out = vec![Primitive::Rect {
        x: r(i),
        y: r(i),
        width: r(w - i * 2.0),
        height: r(h - i * 2.0),
        radius: Some(6.0),
    }];

    // Separador de la cabecera. Solo si hay filas debajo: una tabla vacia es
    // una caja con su nombre, sin raya suelta.
    if rows > 0 {
        out.push(Primitive::Line {
            d: format!(
                "M {} {} L {} {}",
                r(i),
                r(TABLE_HEADER_HEIGHT),
                r(w - i),
                r(TABLE_HEADER_HEIGHT)
            ),
        });
    }

    // Lineas entre filas; la ultima fila no lleva, el marco ya la cierra.
    for index in 1..rows {
        let y = TABLE_HEADER_HEIGHT + index as f64 * TABLE_ROW_HEIGHT;
        if y >= h - i {
            break;
        }
        out.push(Primitive::Line {
            d: format!("M {} {} L {} {}", r(i), r(y), r(w - i), r(y)),
        });
    }

    out
}

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
        // La tabla depende de sus columnas; `table_primitives` la dibuja. Aqui
        // se devuelve el marco para que un nodo mal formado siga siendo
        // visible en vez de desaparecer.
        NodeShape::Table => table_primitives(0, w, h),

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

/// Formas de contorno fijo, las que dibuja `primitives`. La tabla queda
/// fuera a proposito: su contorno depende de los datos.
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
    fn una_tabla_de_tres_columnas_tiene_marco_cabecera_y_separadores() {
        let p = table_primitives(3, 220.0, table_height(3));

        // marco + separador de cabecera + 2 lineas entre 3 filas
        assert_eq!(p.len(), 4);
        assert!(matches!(p[0], Primitive::Rect { .. }));
        assert!(p[1..].iter().all(|x| !x.is_filled()));
    }

    #[test]
    fn una_tabla_sin_columnas_es_solo_el_marco() {
        let p = table_primitives(0, 220.0, TABLE_HEADER_HEIGHT);

        assert_eq!(p.len(), 1);
        assert!(matches!(p[0], Primitive::Rect { .. }));
    }

    #[test]
    fn una_tabla_de_una_columna_lleva_cabecera_pero_ningun_separador() {
        assert_eq!(table_primitives(1, 220.0, table_height(1)).len(), 2);
    }

    #[test]
    fn el_alto_crece_una_fila_por_columna() {
        assert_eq!(table_height(0), TABLE_HEADER_HEIGHT);
        assert_eq!(table_height(4) - table_height(3), TABLE_ROW_HEIGHT);
    }

    #[test]
    fn las_filas_se_apilan_bajo_la_cabecera_sin_huecos() {
        let a = table_row_rect(0, 220.0);
        let b = table_row_rect(1, 220.0);

        assert_eq!(a.y, TABLE_HEADER_HEIGHT);
        assert_eq!(b.y, a.bottom());
        assert_eq!(a.width, 220.0);
    }

    #[test]
    fn las_filas_que_no_caben_no_dibujan_separador() {
        // Caja mas baja de lo que piden las columnas: nada debe salirse.
        let p = table_primitives(10, 220.0, TABLE_HEADER_HEIGHT + TABLE_ROW_HEIGHT * 2.0);

        assert!(p.len() < 11);
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
