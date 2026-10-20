//! Diagramas de arranque.
//!
//! Una pantalla en blanco no dice que se puede hacer con el programa. Estas
//! son siete respuestas a esa pregunta, una por tipo de diagrama, pensadas
//! para abrirse y empezar a cambiarlas.
//!
//! Van escritas en **codigo**, no como documentos guardados, y entran por el
//! mismo importador que cualquier otro texto. Tiene tres consecuencias
//! buenas:
//!
//! - Se leen y se corrigen aqui mismo, sin abrir la aplicacion.
//! - Llegan sin geometria, asi que las coloca el auto-layout como a todo lo
//!   importado: no hay posiciones a mano que se queden desfasadas.
//! - Son una prueba viva de los parsers. Si uno deja de entender algo, la
//!   plantilla que lo usa falla, y falla con un ejemplo realista en lugar de
//!   con un caso de laboratorio.
//!
//! Cada una cubre un dialecto distinto a proposito, para que entre las siete
//! pasen por todos los importadores que hay.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::model::DiagramKind;
use crate::parse::SourceLanguage;

/// Un diagrama de ejemplo listo para abrir.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/model/generated/")]
pub struct Template {
    /// Estable: el frontend lo usa como clave y no se muestra.
    pub id: String,
    pub name: String,
    /// Una linea diciendo que ensena, que es lo que decide cual abrir.
    pub description: String,
    pub kind: DiagramKind,
    pub language: SourceLanguage,
    pub source: String,
}

/// Todas, en el orden en que se ofrecen.
pub fn all() -> Vec<Template> {
    DEFINITIONS
        .iter()
        .map(|(id, name, description, kind, language, source)| Template {
            id: (*id).to_string(),
            name: (*name).to_string(),
            description: (*description).to_string(),
            kind: *kind,
            language: *language,
            source: source.trim_start().to_string(),
        })
        .collect()
}

type Definition = (
    &'static str,
    &'static str,
    &'static str,
    DiagramKind,
    SourceLanguage,
    &'static str,
);

const DEFINITIONS: &[Definition] = &[
    (
        "er-tienda",
        "Tienda en linea",
        "Clientes, pedidos y lineas, con sus claves y cardinalidades.",
        DiagramKind::Er,
        SourceLanguage::Dbml,
        TIENDA,
    ),
    (
        "arq-tres-capas",
        "Aplicacion de tres capas",
        "Presentacion, servicio y datos, con lo que habla con que.",
        DiagramKind::Architecture,
        SourceLanguage::PlantUml,
        TRES_CAPAS,
    ),
    (
        "clases-dominio",
        "Modelo de dominio",
        "Herencia, composicion y atributos con su visibilidad.",
        DiagramKind::Class,
        SourceLanguage::Mermaid,
        DOMINIO,
    ),
    (
        "secuencia-login",
        "Inicio de sesion",
        "Mensajes, activaciones y una alternativa segun el resultado.",
        DiagramKind::Sequence,
        SourceLanguage::PlantUml,
        LOGIN,
    ),
    (
        "estado-pedido",
        "Ciclo de vida de un pedido",
        "Estados, transiciones con guarda y un compuesto por dentro.",
        DiagramKind::State,
        SourceLanguage::PlantUml,
        CICLO_PEDIDO,
    ),
    (
        "actividad-aprobacion",
        "Proceso de aprobacion",
        "Condiciones, un bucle y dos ramas que corren a la vez.",
        DiagramKind::Activity,
        SourceLanguage::PlantUml,
        APROBACION,
    ),
    (
        "casos-reservas",
        "Sistema de reservas",
        "Actores, la frontera del sistema y una relacion «include».",
        DiagramKind::UseCase,
        SourceLanguage::PlantUml,
        RESERVAS,
    ),
];

const TIENDA: &str = r#"
Table clientes {
  id integer [pk, increment]
  nombre varchar(120) [not null]
  correo varchar(160) [unique]
  alta timestamp
}

Table pedidos {
  id integer [pk, increment]
  cliente_id integer [not null]
  fecha timestamp [not null]
  estado varchar(20)
}

Table lineas {
  id integer [pk, increment]
  pedido_id integer [not null]
  producto_id integer [not null]
  cantidad integer [not null]
  precio decimal(10,2)
}

Table productos {
  id integer [pk, increment]
  nombre varchar(160) [not null]
  precio decimal(10,2)
}

Ref: pedidos.cliente_id > clientes.id
Ref: lineas.pedido_id > pedidos.id
Ref: lineas.producto_id > productos.id
"#;

const TRES_CAPAS: &str = r#"
@startuml
left to right direction

actor "Usuario" as usuario

package "Presentacion" {
  [Interfaz web] as web
  [API REST] as api
}

package "Servicio" {
  component "Reglas de negocio" as reglas
  component "Trabajos en diferido" as trabajos
}

package "Datos" {
  database "PostgreSQL" as base
  queue "Cola de eventos" as cola
}

usuario --> web : HTTPS
web --> api
api --> reglas
reglas --> base
reglas ..> cola : publica
cola --> trabajos
trabajos --> base
@enduml
"#;

const DOMINIO: &str = r#"
classDiagram
    class Documento {
        #String titulo
        #Date creado
        +renombrar(String) void
        +resumen() String
    }

    class Informe {
        -String autor
        +firmar() void
    }

    class Plantilla {
        -boolean publica
        +instanciar() Documento
    }

    class Seccion {
        +String titulo
        +String cuerpo
    }

    Documento <|-- Informe
    Documento <|-- Plantilla
    Documento *-- Seccion
"#;

const LOGIN: &str = r#"
@startuml
participant "Navegador" as nav
participant "Servicio" as svc
participant "Directorio" as dir

nav -> svc ++ : credenciales
svc -> dir ++ : comprobar
dir --> svc -- : resultado

alt credenciales validas
  svc --> nav : token de sesion
else no coinciden
  svc --> nav : error 401
end

deactivate svc
@enduml
"#;

const CICLO_PEDIDO: &str = r#"
@startuml
[*] --> Borrador
Borrador : el cliente aun lo edita

Borrador --> Confirmado : confirmar
Confirmado --> Cancelado : cancelar

state Confirmado {
  [*] --> Cobrando
  Cobrando --> Cobrado : pago aceptado
  Cobrando --> Rechazado : pago rechazado
  Cobrado --> [*]
}

Confirmado --> Enviado : preparar
Enviado --> Entregado : acuse de recibo
Entregado --> [*]
Cancelado --> [*]
@enduml
"#;

const APROBACION: &str = r#"
@startuml
start
:Recibir solicitud;

if (importe mayor al limite?) then (si)
  :Pedir visto bueno;

  while (falta alguna firma?) is (si)
    :Reclamar firma;
  endwhile (no)
else (no)
  :Aprobar sin mas tramite;
endif

fork
  :Avisar al solicitante;
fork again
  :Anotar en el expediente;
end fork

:Cerrar solicitud;
stop
@enduml
"#;

const RESERVAS: &str = r#"
@startuml
left to right direction

:Cliente: as cliente
:Recepcion: as recepcion

rectangle "Sistema de reservas" {
  (Consultar disponibilidad) as consultar
  (Reservar) as reservar
  (Cancelar reserva) as cancelar
  (Cobrar) as cobrar
}

cliente --> consultar
cliente --> reservar
cliente --> cancelar
recepcion --> cobrar
reservar ..> cobrar
@enduml
"#;

#[cfg(test)]
mod tests;
