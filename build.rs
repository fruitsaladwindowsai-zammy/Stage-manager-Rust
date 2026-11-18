use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=assets/app.manifest");
    println!("cargo:rerun-if-env-changed=STAGE_MANAGER_EMBED_PATH");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR missing"));
    let payload_rs = out_dir.join("embedded_payload.rs");

    let mut embedded_path: Option<PathBuf> = None;
    if let Ok(path) = std::env::var("STAGE_MANAGER_EMBED_PATH") {
        let abs = PathBuf::from(&path);
        let resolved = if abs.is_absolute() {
            abs
        } else {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(abs)
        };
        if resolved.is_file() {
            println!("cargo:rerun-if-changed={}", resolved.display());
            embedded_path = Some(resolved);
        }
    }

    let code = if let Some(p) = embedded_path {
        let escaped = p
            .display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        format!("pub const EMBEDDED_STAGE_MANAGER: Option<&[u8]> = Some(include_bytes!(\"{escaped}\"));\n")
    } else {
        "pub const EMBEDDED_STAGE_MANAGER: Option<&[u8]> = None;\n".to_string()
    };

    std::fs::write(payload_rs, code).expect("Failed to write embedded_payload.rs");
}
