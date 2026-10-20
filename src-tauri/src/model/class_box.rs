//! Caja de un diagrama de clases.
//!
//! Tiene la misma forma de problema que una tabla entidad-relacion —un nodo
//! cuyo contorno depende de su contenido— pero con tres compartimentos en
//! vez de dos: el nombre, los atributos y las operaciones.
//!
//! Los parametros de una operacion se guardan **tal como se escribieron**, en
//! una sola cadena. Partirlos en tipos y nombres seria otro lenguaje que
//! mantener, y lo unico que esta herramienta hace con ellos es dibujarlos.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Quien puede ver un miembro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
    /// Visible dentro del paquete. UML lo escribe con una tilde.
    Package,
}

impl Visibility {
    /// El signo con el que UML la escribe delante del nombre.
    pub fn sigil(self) -> char {
        match self {
            Visibility::Public => '+',
            Visibility::Private => '-',
            Visibility::Protected => '#',
            Visibility::Package => '~',
        }
    }

    /// Que visibilidad declara un signo, si es que declara alguna.
    pub fn from_sigil(sigil: char) -> Option<Self> {
        match sigil {
            '+' => Some(Visibility::Public),
            '-' => Some(Visibility::Private),
            '#' => Some(Visibility::Protected),
            '~' => Some(Visibility::Package),
            _ => None,
        }
    }
}

/// Un atributo o una operacion.
///
/// `id` es estable y distinto del nombre, igual que en una columna: las
/// aristas pueden apuntar a el y renombrar no rompe nada.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct ClassMember {
    pub id: String,
    pub name: String,

    #[serde(default)]
    pub visibility: Visibility,

    /// Tipo del atributo, o lo que devuelve la operacion. Puede ir vacio.
    #[serde(default)]
    pub type_name: String,

    /// Parametros de una operacion, tal como se escribieron y sin los
    /// parentesis. Vacio en un atributo, y tambien en una operacion que no
    /// recibe nada.
    #[serde(default)]
    pub parameters: String,

    /// Se subraya al dibujarlo, que es como UML marca lo estatico.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_static: bool,

    /// Se escribe en cursiva, que es como UML marca lo abstracto.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_abstract: bool,
}

impl ClassMember {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        ClassMember {
            id: id.into(),
            name: name.into(),
            visibility: Visibility::default(),
            type_name: String::new(),
            parameters: String::new(),
            is_static: false,
            is_abstract: false,
        }
    }

    pub fn with_visibility(mut self, visibility: Visibility) -> Self {
        self.visibility = visibility;
        self
    }

    pub fn with_type(mut self, type_name: impl Into<String>) -> Self {
        self.type_name = type_name.into();
        self
    }

    pub fn with_parameters(mut self, parameters: impl Into<String>) -> Self {
        self.parameters = parameters.into();
        self
    }

    /// Como se escribe el miembro dentro de la caja.
    ///
    /// Es la **unica** definicion de ese texto: la usan el lienzo y el
    /// exportador, y el fichero dorado vigila que la copia de TypeScript
    /// siga coincidiendo. Un atributo y una operacion solo se diferencian en
    /// los parentesis, que la operacion lleva siempre aunque no reciba nada.
    pub fn text(&self, is_operation: bool) -> String {
        let mut out = String::with_capacity(32);
        out.push(self.visibility.sigil());
        out.push(' ');
        out.push_str(&self.name);

        if is_operation {
            out.push('(');
            out.push_str(&self.parameters);
            out.push(')');
        }

        if !self.type_name.is_empty() {
            out.push_str(": ");
            out.push_str(&self.type_name);
        }

        out
    }
}

/// Lo que lleva dentro una caja de clase.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct ClassBox {
    /// `interface`, `enumeration`, `abstract`... Se dibuja encima del nombre
    /// entre dobles angulos, y sin ellos: los pone el dibujo.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub stereotype: Option<String>,

    #[serde(default)]
    pub attributes: Vec<ClassMember>,

    #[serde(default)]
    pub operations: Vec<ClassMember>,
}

impl ClassBox {
    pub fn new() -> Self {
        ClassBox::default()
    }

    pub fn with_stereotype(mut self, stereotype: impl Into<String>) -> Self {
        self.stereotype = Some(stereotype.into());
        self
    }

    pub fn with_attributes(mut self, attributes: Vec<ClassMember>) -> Self {
        self.attributes = attributes;
        self
    }

    pub fn with_operations(mut self, operations: Vec<ClassMember>) -> Self {
        self.operations = operations;
        self
    }

    /// `true` si hay que hacer sitio para el estereotipo encima del nombre.
    pub fn has_stereotype(&self) -> bool {
        self.stereotype
            .as_deref()
            .is_some_and(|s| !s.trim().is_empty())
    }

    /// Un miembro por su id, mire en el compartimento que mire.
    pub fn member(&self, id: &str) -> Option<&ClassMember> {
        self.attributes
            .iter()
            .chain(&self.operations)
            .find(|m| m.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_visibilidad_tiene_su_signo_y_vuelve_de_el() {
        for visibility in [
            Visibility::Public,
            Visibility::Private,
            Visibility::Protected,
            Visibility::Package,
        ] {
            assert_eq!(Visibility::from_sigil(visibility.sigil()), Some(visibility));
        }

        assert_eq!(Visibility::from_sigil('x'), None);
    }

    #[test]
    fn un_atributo_se_escribe_con_su_tipo() {
        let member = ClassMember::new("a1", "nombre")
            .with_visibility(Visibility::Private)
            .with_type("String");

        assert_eq!(member.text(false), "- nombre: String");
    }

    #[test]
    fn un_atributo_sin_tipo_no_lleva_los_dos_puntos() {
        assert_eq!(ClassMember::new("a1", "nombre").text(false), "+ nombre");
    }

    #[test]
    fn una_operacion_lleva_parentesis_aunque_no_reciba_nada() {
        let member = ClassMember::new("o1", "guardar");

        assert_eq!(member.text(true), "+ guardar()");
    }

    #[test]
    fn una_operacion_escribe_sus_parametros_y_lo_que_devuelve() {
        let member = ClassMember::new("o1", "buscar")
            .with_visibility(Visibility::Protected)
            .with_parameters("id: int, activo: bool")
            .with_type("Pedido");

        assert_eq!(member.text(true), "# buscar(id: int, activo: bool): Pedido");
    }

    #[test]
    fn un_estereotipo_en_blanco_no_ocupa_sitio() {
        assert!(!ClassBox::new().has_stereotype());
        assert!(!ClassBox::new().with_stereotype("  ").has_stereotype());
        assert!(
            ClassBox::new()
                .with_stereotype("interface")
                .has_stereotype()
        );
    }

    #[test]
    fn se_busca_un_miembro_en_los_dos_compartimentos() {
        let box_ = ClassBox::new()
            .with_attributes(vec![ClassMember::new("a1", "nombre")])
            .with_operations(vec![ClassMember::new("o1", "guardar")]);

        assert_eq!(box_.member("a1").map(|m| m.name.as_str()), Some("nombre"));
        assert_eq!(box_.member("o1").map(|m| m.name.as_str()), Some("guardar"));
        assert!(box_.member("fantasma").is_none());
    }
}
