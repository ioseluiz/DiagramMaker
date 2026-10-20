//! Un diagrama bien escrito no tiene que decir nada.
//!
//! Las comprobaciones por tipo solo sirven si quien las lee se las cree, y
//! para eso tienen que callarse con lo normal. Aqui entra un ejemplo realista
//! de cada dialecto por la puerta de siempre —el importador— y se exige que
//! el validador no suelte ni un aviso.
//!
//! Es la red contra el fallo mas probable de esa parte: una regla razonable
//! sobre el papel que en la practica se dispara con la mitad de los
//! diagramas y se aprende a ignorar.

use diagrammaker_lib::model::validate;
use diagrammaker_lib::parse::{dbml, mermaid, plantuml};

/// Importa y valida, devolviendo los codigos que salgan.
fn quejas(document: diagrammaker_lib::parse::ImportResult) -> Vec<String> {
    assert!(
        document.ok(),
        "el ejemplo no deberia fallar al importar: {:#?}",
        document.diagnostics
    );

    validate::validate(&document.document)
        .diagnostics
        .iter()
        .map(|d| format!("[{}] {}", d.code, d.message))
        .collect()
}

#[test]
fn un_entidad_relacion_de_dbml_no_se_queja() {
    let quejas = quejas(dbml::import(
        r#"
Table clientes {
  id integer [pk]
  nombre varchar(120)
}

Table pedidos {
  id integer [pk]
  cliente_id integer
  total decimal(10,2)
}

Ref: pedidos.cliente_id > clientes.id
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}

#[test]
fn un_entidad_relacion_de_mermaid_no_se_queja() {
    let quejas = quejas(mermaid::import(
        r#"
erDiagram
    CLIENTE ||--o{ PEDIDO : coloca

    CLIENTE {
        int id PK
        string nombre
    }
    PEDIDO {
        int numero PK
        int cliente_id FK
    }
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}

#[test]
fn una_arquitectura_no_se_queja() {
    let quejas = quejas(plantuml::import(
        r#"
@startuml
actor "Cliente" as cliente
package "Capa publica" {
  [Portal web] as web
  component "API REST" as api
}
database "PostgreSQL" as base

cliente --> web : HTTPS
web --> api
api --> base
@enduml
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}

#[test]
fn un_diagrama_de_clases_no_se_queja() {
    let quejas = quejas(mermaid::import(
        r#"
classDiagram
    class Pedido {
        +int numero
        +total() decimal
    }
    class PedidoWeb {
        +String referencia
    }
    Pedido <|-- PedidoWeb
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}

#[test]
fn una_secuencia_no_se_queja() {
    let quejas = quejas(plantuml::import(
        r#"
@startuml
participant Cliente
participant Tienda

Cliente -> Tienda ++ : comprar
Tienda --> Cliente -- : confirmacion

alt hay stock
  Cliente -> Tienda : pagar
else no hay
  Cliente -> Tienda : cancelar
end
@enduml
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}

#[test]
fn una_maquina_de_estados_no_se_queja() {
    let quejas = quejas(plantuml::import(
        r#"
@startuml
[*] --> Nuevo
Nuevo --> Pagado : confirmar

state Pagado {
  [*] --> Cobrando
  Cobrando --> Cobrado
  Cobrado --> [*]
}

Pagado --> Enviado
Enviado --> [*]
@enduml
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}

#[test]
fn un_diagrama_de_casos_de_uso_no_se_queja() {
    let quejas = quejas(plantuml::import(
        r#"
@startuml
left to right direction

:Cliente: as cliente
:Almacen: as almacen

rectangle "Tienda" {
  (Comprar) as comprar
  (Pagar) as pagar
  (Enviar) as enviar
}

cliente --> comprar
cliente --> pagar
comprar ..> pagar
almacen --> enviar
@enduml
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}

#[test]
fn un_diagrama_de_actividad_no_se_queja() {
    let quejas = quejas(plantuml::import(
        r#"
@startuml
start
:Recibir pedido;

if (hay stock?) then (si)
  :Reservar articulos;
else (no)
  :Avisar al cliente;
  stop
endif

fork
  :Enviar correo;
fork again
  :Actualizar inventario;
end fork

while (hay incidencias?) is (si)
  :Resolver incidencia;
endwhile (no)

:Cerrar pedido;
stop
@enduml
"#,
        0,
    ));

    assert!(quejas.is_empty(), "{quejas:#?}");
}
