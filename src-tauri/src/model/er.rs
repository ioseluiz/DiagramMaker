use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Una columna de una tabla.
///
/// `id` es estable y distinto del nombre: las aristas apuntan a el por
/// `Endpoint.port`, de modo que renombrar una columna no rompe sus relaciones.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct ErColumn {
    pub id: String,
    pub name: String,

    /// Tipo tal y como lo escribio el usuario: `varchar(255)`, `integer`,
    /// `post_status`. No se normaliza, porque cada motor tiene los suyos.
    #[serde(default)]
    pub data_type: String,

    #[serde(default)]
    pub primary_key: bool,

    /// Se deduce de las relaciones al importar; sirve para el icono.
    #[serde(default)]
    pub foreign_key: bool,

    /// Las columnas admiten nulos salvo que se diga lo contrario, igual que
    /// en SQL. Una clave primaria nunca es nula.
    #[serde(default = "yes")]
    pub nullable: bool,

    #[serde(default)]
    pub unique: bool,

    #[serde(default)]
    pub increment: bool,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub default_value: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub note: Option<String>,
}

fn yes() -> bool {
    true
}

impl ErColumn {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        ErColumn {
            id: id.into(),
            name: name.into(),
            data_type: String::new(),
            primary_key: false,
            foreign_key: false,
            nullable: true,
            unique: false,
            increment: false,
            default_value: None,
            note: None,
        }
    }

    pub fn with_type(mut self, data_type: impl Into<String>) -> Self {
        self.data_type = data_type.into();
        self
    }

    /// Marca la columna como clave primaria. Una PK no admite nulos, asi que
    /// lo ajusta de paso en lugar de dejar un estado contradictorio.
    pub fn as_primary_key(mut self) -> Self {
        self.primary_key = true;
        self.nullable = false;
        self
    }

    /// Sufijos que se muestran junto al tipo: `PK`, `FK`, `NN`, `U`.
    pub fn badges(&self) -> Vec<&'static str> {
        let mut out = Vec::new();
        if self.primary_key {
            out.push("PK");
        }
        if self.foreign_key {
            out.push("FK");
        }
        if self.unique && !self.primary_key {
            out.push("U");
        }
        if !self.nullable && !self.primary_key {
            out.push("NN");
        }
        out
    }
}

/// Datos de una tabla de un diagrama entidad-relacion.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct ErTable {
    #[serde(default)]
    pub columns: Vec<ErColumn>,

    /// Alias de DBML (`Table users as U`). Se conserva para poder volver a
    /// generar el codigo igual que entro.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub alias: Option<String>,

    /// Esquema o base de datos: el `public` de `public.users`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub schema: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub note: Option<String>,
}

impl ErTable {
    pub fn column(&self, id: &str) -> Option<&ErColumn> {
        self.columns.iter().find(|c| c.id == id)
    }

    pub fn column_mut(&mut self, id: &str) -> Option<&mut ErColumn> {
        self.columns.iter_mut().find(|c| c.id == id)
    }

    /// Busca por nombre, sin distinguir mayusculas: es como se referencian las
    /// columnas en el codigo DBML.
    pub fn column_by_name(&self, name: &str) -> Option<&ErColumn> {
        self.columns
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(name))
    }

    pub fn primary_keys(&self) -> impl Iterator<Item = &ErColumn> {
        self.columns.iter().filter(|c| c.primary_key)
    }
}

/// Cardinalidad de una relacion entre tablas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub enum Cardinality {
    One,
    Many,
}

impl Cardinality {
    /// Como se escribe en una etiqueta de extremo.
    pub fn label(self) -> &'static str {
        match self {
            Cardinality::One => "1",
            Cardinality::Many => "*",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_columna_nueva_admite_nulos() {
        assert!(ErColumn::new("c1", "nombre").nullable);
    }

    #[test]
    fn una_clave_primaria_deja_de_admitir_nulos() {
        let c = ErColumn::new("c1", "id").as_primary_key();

        assert!(c.primary_key);
        assert!(!c.nullable);
    }

    #[test]
    fn las_marcas_salen_en_orden_y_sin_redundancias() {
        let mut c = ErColumn::new("c1", "id").as_primary_key();
        c.unique = true;

        // Una PK ya es unica y no nula: no hace falta repetirlo.
        assert_eq!(c.badges(), ["PK"]);
    }

    #[test]
    fn una_columna_normal_muestra_sus_restricciones() {
        let mut c = ErColumn::new("c1", "email");
        c.unique = true;
        c.nullable = false;

        assert_eq!(c.badges(), ["U", "NN"]);
    }

    #[test]
    fn una_clave_ajena_se_marca() {
        let mut c = ErColumn::new("c1", "user_id");
        c.foreign_key = true;

        assert_eq!(c.badges(), ["FK"]);
    }

    #[test]
    fn busca_columnas_por_nombre_sin_distinguir_mayusculas() {
        let table = ErTable {
            columns: vec![ErColumn::new("c1", "UserId")],
            ..Default::default()
        };

        assert!(table.column_by_name("userid").is_some());
        assert!(table.column_by_name("otra").is_none());
    }

    #[test]
    fn una_columna_sin_nulabilidad_en_el_json_la_recupera_como_true() {
        // `nullable` se omite al serializar cuando es el valor por defecto.
        let c: ErColumn = serde_json::from_str(r#"{"id":"c1","name":"x"}"#).unwrap();

        assert!(c.nullable);
    }

    #[test]
    fn una_tabla_sobrevive_al_viaje_por_json() {
        let table = ErTable {
            columns: vec![
                ErColumn::new("c1", "id")
                    .with_type("integer")
                    .as_primary_key(),
                ErColumn::new("c2", "email").with_type("varchar(255)"),
            ],
            alias: Some("U".into()),
            schema: Some("public".into()),
            note: Some("Usuarios".into()),
        };

        let back: ErTable = serde_json::from_str(&serde_json::to_string(&table).unwrap()).unwrap();

        assert_eq!(back, table);
    }

    #[test]
    fn enumera_las_claves_primarias() {
        let table = ErTable {
            columns: vec![
                ErColumn::new("c1", "a").as_primary_key(),
                ErColumn::new("c2", "b"),
                ErColumn::new("c3", "c").as_primary_key(),
            ],
            ..Default::default()
        };

        assert_eq!(table.primary_keys().count(), 2);
    }
}
