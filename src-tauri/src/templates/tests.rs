use super::*;

use crate::model::validate;
use crate::parse;

/* --- La lista ---------------------------------------------------------------- */

#[test]
fn hay_una_por_cada_tipo_de_diagrama() {
    // Si llega un tipo nuevo, esta prueba recuerda que le falta su ejemplo.
    let mut kinds: Vec<String> = all().iter().map(|t| format!("{:?}", t.kind)).collect();
    kinds.sort();

    assert_eq!(
        kinds,
        [
            "Activity",
            "Architecture",
            "Class",
            "Er",
            "Sequence",
            "State",
            "UseCase",
        ]
    );
}

#[test]
fn los_identificadores_no_se_repiten() {
    let todas = all();
    let mut ids: Vec<&str> = todas.iter().map(|t| t.id.as_str()).collect();
    let total = ids.len();
    ids.sort();
    ids.dedup();

    assert_eq!(ids.len(), total);
}

#[test]
fn todas_tienen_nombre_y_una_linea_que_las_explique() {
    for template in all() {
        assert!(!template.name.trim().is_empty(), "{}", template.id);
        assert!(!template.description.trim().is_empty(), "{}", template.id);
        assert!(!template.source.trim().is_empty(), "{}", template.id);
    }
}

#[test]
fn entre_todas_pasan_por_los_tres_idiomas() {
    // Es el motivo de escribirlas en dialectos distintos: que ninguna rama
    // del importador se quede sin un ejemplo realista.
    let mut languages: Vec<String> = all().iter().map(|t| format!("{:?}", t.language)).collect();
    languages.sort();
    languages.dedup();

    assert_eq!(languages, ["Dbml", "Mermaid", "PlantUml"]);
}

/* --- Que de verdad funcionen --------------------------------------------------- */

#[test]
fn todas_importan_sin_un_solo_diagnostico() {
    for template in all() {
        let result = parse::import(template.language, &template.source, 0);

        assert!(
            result.diagnostics.is_empty(),
            "la plantilla `{}` no importa limpia: {:#?}",
            template.id,
            result.diagnostics
        );
    }
}

#[test]
fn todas_salen_del_tipo_que_dicen_ser() {
    // El tipo lo decide el importador al olfatear el codigo; lo que declara
    // la plantilla tiene que coincidir o la lista estaria mintiendo.
    for template in all() {
        let document = parse::import(template.language, &template.source, 0).document;

        assert_eq!(
            document.kind, template.kind,
            "la plantilla `{}` dice ser {:?} y sale {:?}",
            template.id, template.kind, document.kind
        );
    }
}

#[test]
fn ninguna_se_queja_al_validarse() {
    // Abrir un ejemplo y encontrarse el contador de problemas en rojo seria
    // la peor manera de empezar.
    for template in all() {
        let document = parse::import(template.language, &template.source, 0).document;
        let report = validate::validate(&document);

        assert!(
            report.diagnostics.is_empty(),
            "la plantilla `{}` se queja: {:#?}",
            template.id,
            report.diagnostics
        );
    }
}

#[test]
fn ninguna_llega_vacia_ni_con_geometria() {
    for template in all() {
        let document = parse::import(template.language, &template.source, 0).document;

        assert!(
            document.nodes.len() >= 3,
            "la plantilla `{}` tiene muy poco que ensenar",
            template.id
        );
        assert!(
            document.nodes.iter().all(|n| n.geometry.is_none()),
            "la plantilla `{}` trae posiciones a mano; las pone el auto-layout",
            template.id
        );
    }
}
