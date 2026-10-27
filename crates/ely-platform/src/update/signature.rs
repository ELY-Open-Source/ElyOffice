//! Release manifests carry an ed25519 signature (§4.2).

use anyhow::{Context, Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ring::signature::{ED25519, Ed25519KeyPair, UnparsedPublicKey};

/// The release key's public half; its seed signs in CI.
pub const KEY: &str = "mK2fTwN43B6zYjAJlrMLyz1qTGayGcGZ/fPHokXOHro=";

/// What is signed: the file's name, then its bytes.
fn message(name: &str, bytes: &[u8]) -> Vec<u8> {
    [name.as_bytes(), b"\n", bytes].concat()
}

/// Signs `bytes`, named `name`, with a base64 seed.
pub fn sign(seed: &str, name: &str, bytes: &[u8]) -> Result<String> {
    let seed = STANDARD.decode(seed.trim()).context("the signing key is not base64")?;
    let pair = Ed25519KeyPair::from_seed_unchecked(&seed)
        .map_err(|error| anyhow!("the signing key is no ed25519 seed: {error}"))?;
    Ok(STANDARD.encode(pair.sign(&message(name, bytes))))
}

/// Passes only what `key` signed under this name.
pub(super) fn verify(key: &str, name: &str, bytes: &[u8], signature: &str) -> Result<()> {
    let key = STANDARD.decode(key.trim()).context("the release key is not base64")?;
    let signature = STANDARD
        .decode(signature.trim())
        .with_context(|| format!("{name}'s signature is not base64"))?;
    UnparsedPublicKey::new(&ED25519, key)
        .verify(&message(name, bytes), &signature)
        .map_err(|_| anyhow!("{name} is not signed by the release key"))
}

#[cfg(test)]
mod tests {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use ring::signature::{Ed25519KeyPair, KeyPair as _};

    use super::{KEY, sign, verify};

    const SEED: [u8; 32] = [7; 32];

    fn key() -> String {
        let pair = Ed25519KeyPair::from_seed_unchecked(&SEED).expect("a pair");
        STANDARD.encode(pair.public_key().as_ref())
    }

    #[test]
    fn only_the_signed_name_and_bytes_pass() {
        let seed = STANDARD.encode(SEED);
        let signed = sign(&seed, "ely-doc-windows-x86_64.json", b"{}").expect("signed");
        verify(&key(), "ely-doc-windows-x86_64.json", b"{}", &signed).expect("its own signature");
        let refused = |name: &str, bytes: &[u8], key: &str| verify(key, name, bytes, &signed).is_err();
        assert!(refused("ely-doc-windows-x86_64.json", b"{ }", &key()), "changed bytes");
        assert!(refused("ely-tree-windows-x86_64.json", b"{}", &key()), "another app's name");
        assert!(refused("ely-doc-windows-x86_64.json", b"{}", KEY), "another key");
        assert!(verify(&key(), "ely-doc-windows-x86_64.json", b"{}", "").is_err(), "no signature");
    }

    #[test]
    fn the_release_key_is_an_ed25519_public_key() {
        assert_eq!(STANDARD.decode(KEY).expect("base64").len(), 32);
    }
}
