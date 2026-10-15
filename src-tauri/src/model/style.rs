use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Sobrescrituras visuales de un nodo.
///
/// Todo es `Option`: `None` significa "usa lo que diga el tema". El documento
/// guarda unicamente lo que el usuario cambio a proposito, de modo que cambiar
/// de tema sigue afectando a los nodos que nunca se tocaron.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct NodeStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub fill: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stroke: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stroke_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub bold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub italic: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dashed: Option<bool>,
}

impl NodeStyle {
    pub fn is_default(&self) -> bool {
        self == &NodeStyle::default()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct EdgeStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stroke: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stroke_width: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub text_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub font_size: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub dashed: Option<bool>,
}

impl EdgeStyle {
    pub fn is_default(&self) -> bool {
        self == &EdgeStyle::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_estilo_vacio_se_serializa_a_nulos_y_vuelve_igual() {
        let s = NodeStyle::default();
        let json = serde_json::to_string(&s).unwrap();
        let back: NodeStyle = serde_json::from_str(&json).unwrap();

        assert!(back.is_default());
    }

    #[test]
    fn un_json_parcial_rellena_el_resto_con_none() {
        let s: NodeStyle = serde_json::from_str(r##"{"fill":"#ff0000"}"##).unwrap();

        assert_eq!(s.fill.as_deref(), Some("#ff0000"));
        assert_eq!(s.stroke, None);
        assert!(!s.is_default());
    }
}
