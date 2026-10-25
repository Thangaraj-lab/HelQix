//! helqix-evidence-pack keygen <secret-key-file>
//!     Writes a new development signing key (hex) and prints its public key. Keep the file out of git.
//! helqix-evidence-pack build <out-dir> --key <secret-key-file> --signer <name> --signed-at <claimed time>
//!     Builds the core-runtime reference evidence package and signs it.
//! helqix-evidence-pack reexecute <package-dir>
//!     Replays from the package's freeze and compares with the recorded final state hash (Bitwise).
use std::{fs, path::Path, process::ExitCode};

use ed25519_dalek::SigningKey;
use helqix_evidence_pack::{generate_key, public_key_hex, reexecute, reference_files, write_package};

fn load_key(path: &str) -> Result<SigningKey, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("cannot read key: {e}"))?;
    let t = text.trim();
    let bytes: Vec<u8> = (0..t.len()).step_by(2).map(|i| u8::from_str_radix(t.get(i..i + 2).unwrap_or("zz"), 16)).collect::<Result<_, _>>().map_err(|_| "key file is not hex".to_owned())?;
    let seed: [u8; 32] = bytes.try_into().map_err(|_| "key must be 32 bytes".to_owned())?;
    Ok(SigningKey::from_bytes(&seed))
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let result = match a.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["keygen", path] => generate_key().map_err(|e| e.to_string()).and_then(|k| {
            if Path::new(path).exists() { return Err(format!("{path} exists; refusing to overwrite a key")); }
            fs::write(path, k.to_bytes().iter().map(|b| format!("{b:02x}")).collect::<String>()).map_err(|e| e.to_string())?;
            println!("development key written to {path} (do not commit)\npublic key: {}", public_key_hex(&k));
            Ok(true)
        }),
        ["build", out, "--key", key, "--signer", signer, "--signed-at", at] => load_key(key).and_then(|k| {
            let (files, statement) = reference_files().map_err(|e| e.to_string())?;
            let s = write_package(Path::new(out), files, &statement, signer, at, &k).map_err(|e| e.to_string())?;
            println!("package {out}\nmerkle root {}\nmanifest sha256 {}\npublic key {}\nlabel {}", s.merkle_root, s.manifest_sha256, s.public_key, statement.label);
            Ok(true)
        }),
        ["reexecute", dir] => reexecute(Path::new(dir)).map_err(|e| e.to_string()).map(|ok| { println!("re-execution from freeze: {}", if ok { "PASS (bit-identical final state hash; Bitwise)" } else { "FAIL (final state hash differs)" }); ok }),
        _ => Err("usage: helqix-evidence-pack keygen <key-file> | build <out-dir> --key <key-file> --signer <name> --signed-at <time> | reexecute <package-dir>".into()),
    };
    match result { Ok(true) => ExitCode::SUCCESS, Ok(false) => ExitCode::from(2), Err(e) => { eprintln!("{e}"); ExitCode::from(1) } }
}
