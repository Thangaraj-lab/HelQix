//! Writes the JSON Schema of every contract type to `<out-dir>/<Type>.schema.json`.
//! Usage: `helqix-contracts-schema <out-dir>` (the committed copy is `15-sdk/platform/contracts/schema`).
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(dir) = std::env::args().nth(1) else { eprintln!("usage: helqix-contracts-schema <out-dir>"); return ExitCode::from(1) };
    if let Err(e) = std::fs::create_dir_all(&dir) { eprintln!("{dir}: {e}"); return ExitCode::from(1); }
    for (name, schema) in helqix_contracts::schemas() {
        let path = std::path::Path::new(&dir).join(format!("{name}.schema.json"));
        let text = serde_json::to_string_pretty(&schema).expect("schema serializes") + "\n";
        if let Err(e) = std::fs::write(&path, text) { eprintln!("{}: {e}", path.display()); return ExitCode::from(1); }
        println!("{}", path.display());
    }
    ExitCode::SUCCESS
}
