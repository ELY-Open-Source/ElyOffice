//! The system's CA store, checked by webpki.

use std::io;

use rustls_pki_types::{CertificateDer, UnixTime};
use webpki::{ALL_VERIFICATION_ALGS, EndEntityCert, ExtendedKeyUsageValidator, KeyPurposeIdIter, anchor_from_trusted_cert};

use super::Trust;

/// Any extended key usage, as macOS and Windows accept.
struct AnyUsage;

impl ExtendedKeyUsageValidator for AnyUsage {
    fn validate(&self, iter: KeyPurposeIdIter<'_, '_>) -> Result<(), webpki::Error> {
        iter.map(|purpose| purpose.map(drop)).collect()
    }
}

/// `anchors` replaces the system's roots; tests only.
pub fn check(chain: &[Vec<u8>], anchors: Option<&[Vec<u8>]>) -> io::Result<Trust> {
    let Some((leaf, rest)) = chain.split_first() else {
        return Err(io::Error::other("a signature with no certificate"));
    };
    let roots = match anchors {
        Some(anchors) => anchors.iter().map(|der| CertificateDer::from(der.clone())).collect(),
        None => system()?,
    };
    // A trusted root as the signer, as on macOS.
    if roots.iter().any(|root| root.as_ref() == leaf.as_slice()) {
        return Ok(Trust::Trusted);
    }
    let mut anchors = Vec::with_capacity(roots.len());
    for root in &roots {
        match anchor_from_trusted_cert(root) {
            Ok(anchor) => anchors.push(anchor),
            Err(error) => log::warn!("trust: a system root does not parse: {error}"),
        }
    }
    let leaf = CertificateDer::from(leaf.as_slice());
    let leaf = EndEntityCert::try_from(&leaf).map_err(|error| io::Error::other(format!("the signer's certificate: {error}")))?;
    let rest: Vec<CertificateDer> = rest.iter().map(|der| CertificateDer::from(der.as_slice())).collect();
    Ok(match leaf.verify_for_usage(ALL_VERIFICATION_ALGS, &anchors, &rest, UnixTime::now(), AnyUsage, None, None) {
        Ok(_) => Trust::Trusted,
        Err(why) => Trust::Untrusted(why.to_string()),
    })
}

/// The roots the system trusts, as OpenSSL finds them.
fn system() -> io::Result<Vec<CertificateDer<'static>>> {
    let found = rustls_native_certs::load_native_certs();
    for error in &found.errors {
        log::warn!("trust: system roots: {error}");
    }
    if found.certs.is_empty() {
        return Err(io::Error::other("the system lists no root certificates"));
    }
    log::info!("trust: {} system roots", found.certs.len());
    Ok(found.certs)
}

#[cfg(test)]
mod tests {
    use super::super::tests::{leaf, root, signer};
    use super::{Trust, check};

    #[test]
    fn a_self_signed_signer_is_not_trusted_by_the_system() {
        assert!(matches!(check(&[signer()], None).expect("checks"), Trust::Untrusted(why) if !why.is_empty()));
    }

    #[test]
    fn a_chain_to_a_trusted_root_is_trusted() {
        assert_eq!(check(&[leaf()], Some(&[root()])).expect("checks"), Trust::Trusted);
        assert_eq!(check(&[signer()], Some(&[signer()])).expect("checks"), Trust::Trusted);
    }

    #[test]
    fn a_chain_to_another_root_is_not() {
        assert!(matches!(check(&[leaf()], Some(&[signer()])).expect("checks"), Trust::Untrusted(_)));
    }
}
