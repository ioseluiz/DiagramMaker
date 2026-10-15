//! Genera el PNG maestro del icono a partir de `assets/icon.svg`.
//!
//! El SVG es la fuente; de este PNG salen despues todos los tamanos con
//! `npm run tauri icon`. Se rasteriza con el mismo `resvg` que usa la
//! exportacion, asi que no hace falta ninguna herramienta de imagen aparte.
//!
//! ```sh
//! cargo run --manifest-path src-tauri/Cargo.toml --example icon
//! ```

fn main() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let svg = std::fs::read_to_string(dir.join("icon.svg")).expect("falta assets/icon.svg");

    // El SVG mide 1024; a escala 1 el PNG sale con ese mismo tamano, que es
    // el maestro del que `tauri icon` deriva el resto.
    let png = diagrammaker_lib::render::raster::to_png(&svg, 1.0).expect("no se pudo rasterizar");

    let out = dir.join("icon.png");
    std::fs::write(&out, png).expect("no se pudo escribir el PNG");

    println!("icono escrito en {}", out.display());
}
