//! `sign-manifest FILE...`: writes FILE.sig with `ELY_UPDATE_KEY`.

use std::path::Path;

use anyhow::{Context, Result, ensure};

fn main() -> Result<()> {
    let seed = std::env::var("ELY_UPDATE_KEY").context("ELY_UPDATE_KEY holds no signing key")?;
    let files: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(!files.is_empty(), "usage: sign-manifest FILE...");
    for file in files {
        let path = Path::new(&file);
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .with_context(|| format!("{} has no name", path.display()))?;
        let bytes = std::fs::read(path).with_context(|| format!("cannot read {}", path.display()))?;
        let signed = format!("{}.sig", path.display());
        let signature = ely_platform::update::sign(&seed, name, &bytes)?;
        std::fs::write(&signed, signature).with_context(|| format!("cannot write {signed}"))?;
        println!("{signed}");
    }
    Ok(())
}
