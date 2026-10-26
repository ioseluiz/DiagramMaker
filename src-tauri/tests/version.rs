//! La version esta escrita en tres sitios y tiene que decir lo mismo.
//!
//! `Cargo.toml` la usa el binario, `tauri.conf.json` la usa el empaquetador
//! para nombrar el `.msi` y el `.dmg`, y `package.json` la ve quien mire el
//! repositorio. Publicar es poner una etiqueta y dejar que el flujo de
//! Actions construya: si las tres no coinciden, salen paquetes con un numero
//! y una etiqueta con otro, y eso no se arregla sin retirar la publicacion.
//!
//! Por eso esto es una prueba y no una nota: subir la version a mano en tres
//! archivos se olvida, y cuando se olvida ya esta publicado.

/// Lee el valor de `"version"` de un JSON.
fn version_de(ruta: &str) -> String {
    let texto = std::fs::read_to_string(ruta).unwrap_or_else(|e| panic!("{ruta}: {e}"));
    let json: serde_json::Value =
        serde_json::from_str(&texto).unwrap_or_else(|e| panic!("{ruta}: {e}"));

    json["version"]
        .as_str()
        .unwrap_or_else(|| panic!("{ruta}: no tiene `version`"))
        .to_string()
}

#[test]
fn la_version_coincide_en_los_tres_sitios() {
    // Las rutas son relativas a `src-tauri`, que es donde corre la prueba.
    let cargo = env!("CARGO_PKG_VERSION");
    let tauri = version_de("tauri.conf.json");
    let paquete = version_de("../package.json");

    assert_eq!(
        cargo, tauri,
        "Cargo.toml dice {cargo} y tauri.conf.json dice {tauri}"
    );
    assert_eq!(
        cargo, paquete,
        "Cargo.toml dice {cargo} y package.json dice {paquete}"
    );
}

#[test]
fn la_version_tiene_la_forma_de_una_etiqueta() {
    // El flujo de publicacion se dispara con `v*` y nombra la release con la
    // etiqueta. Un `0.1.1-pre` o un `0.1` romperian esa correspondencia.
    let version = env!("CARGO_PKG_VERSION");
    let partes: Vec<&str> = version.split('.').collect();

    assert_eq!(partes.len(), 3, "`{version}` no son tres numeros");
    for parte in partes {
        assert!(
            !parte.is_empty() && parte.chars().all(|c| c.is_ascii_digit()),
            "`{version}` tiene un tramo que no es un numero: `{parte}`"
        );
    }
}
