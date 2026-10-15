use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const ORIGIN: Point = Point { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Point { x, y }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub fn new(width: f64, height: f64) -> Self {
        Size { width, height }
    }
}

/// Caja del nodo en coordenadas del diagrama. El origen es la esquina superior
/// izquierda y el eje Y crece hacia abajo, igual que en SVG y en React Flow, de
/// modo que ninguna de las dos capas tiene que invertir nada.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn origin(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn size(&self) -> Size {
        Size::new(self.width, self.height)
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }

    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Caja minima que contiene a ambas. Base del calculo del `viewBox` al
    /// exportar.
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }

    pub fn inflate(&self, by: f64) -> Rect {
        Rect::new(
            self.x - by,
            self.y - by,
            self.width + by * 2.0,
            self.height + by * 2.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_cubre_ambas_cajas() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(20.0, -5.0, 10.0, 10.0);

        assert_eq!(a.union(&b), Rect::new(0.0, -5.0, 30.0, 15.0));
    }

    #[test]
    fn union_es_conmutativa() {
        let a = Rect::new(3.0, 4.0, 10.0, 2.0);
        let b = Rect::new(-1.0, 0.0, 5.0, 50.0);

        assert_eq!(a.union(&b), b.union(&a));
    }

    #[test]
    fn inflate_crece_en_las_cuatro_direcciones() {
        let r = Rect::new(10.0, 10.0, 4.0, 4.0).inflate(2.0);

        assert_eq!(r, Rect::new(8.0, 8.0, 8.0, 8.0));
    }

    #[test]
    fn center_es_el_punto_medio() {
        assert_eq!(
            Rect::new(0.0, 0.0, 10.0, 20.0).center(),
            Point::new(5.0, 10.0)
        );
    }
}
