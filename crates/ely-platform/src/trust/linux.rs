//! The system's CA store, by webpki or the base policy.

use std::io;

use rustls_pki_types::{CertificateDer, UnixTime};
use webpki::{ALL_VERIFICATION_ALGS, EndEntityCert, ExtendedKeyUsageValidator, KeyPurposeIdIter, anchor_from_trusted_cert};
use x509_parser::prelude::{ParsedExtension, X509Certificate, X509Extension, parse_x509_certificate};

use super::Trust;

/// Any extended key usage, as macOS and Windows accept.
struct AnyUsage;

impl ExtendedKeyUsageValidator for AnyUsage {
    fn validate(&self, mut iter: KeyPurposeIdIter<'_, '_>) -> Result<(), webpki::Error> {
        iter.try_for_each(|purpose| purpose.map(drop))
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
    // webpki takes neither a root nor a CA as signer.
    if roots.iter().any(|root| root.as_ref() == leaf.as_slice()) {
        return Ok(base(leaf, rest, &roots));
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
    let pool: Vec<CertificateDer> = rest.iter().map(|der| CertificateDer::from(der.as_slice())).collect();
    Ok(match leaf.verify_for_usage(ALL_VERIFICATION_ALGS, &anchors, &pool, UnixTime::now(), AnyUsage, None, None) {
        Ok(_) => Trust::Trusted,
        Err(webpki::Error::CaUsedAsEndEntity) => base(&chain[0], rest, &roots),
        Err(why) => Trust::Untrusted(why.to_string()),
    })
}

/// The basic X.509 policy, as Windows and macOS apply it.
fn base(leaf: &[u8], rest: &[Vec<u8>], roots: &[CertificateDer]) -> Trust {
    let checked = (|| {
        let signer = parse(leaf).map_err(|error| format!("the signer's certificate: {error}"))?;
        let pool = rest.iter().map(|der| parse(der)).collect::<Result<Vec<_>, _>>()?;
        let mut anchors = Vec::with_capacity(roots.len());
        for root in roots {
            match parse(root) {
                Ok(anchor) => anchors.push(anchor),
                Err(error) => log::warn!("trust: a system root does not parse: {error}"),
            }
        }
        reaches(&signer, &pool, &anchors, 0)
    })();
    match checked {
        Ok(()) => Trust::Trusted,
        Err(why) => Trust::Untrusted(why),
    }
}

/// One DER certificate, parsed.
fn parse(der: &[u8]) -> Result<X509Certificate<'_>, String> {
    parse_x509_certificate(der).map(|(_, cert)| cert).map_err(|error| error.to_string())
}

/// Whether `cert` chains to an anchor; `below` counts CAs.
fn reaches(cert: &X509Certificate, pool: &[X509Certificate], anchors: &[X509Certificate], below: u32) -> Result<(), String> {
    let name = cert.subject();
    if !cert.validity().is_valid() {
        return Err(format!("{name} is outside its validity period"));
    }
    let unknown = |ext: &&X509Extension| ext.critical && matches!(ext.parsed_extension(), ParsedExtension::UnsupportedExtension { .. });
    if let Some(ext) = cert.extensions().iter().find(unknown) {
        return Err(format!("{name} has an unknown critical extension {}", ext.oid));
    }
    // Fails closed: name constraints are not evaluated.
    if cert.name_constraints().map_err(|error| error.to_string())?.is_some() {
        return Err(format!("{name} has name constraints"));
    }
    if anchors.iter().any(|anchor| anchor.as_raw() == cert.as_raw()) {
        return Ok(());
    }
    if below > MAX_DEPTH {
        return Err("the chain is too long".into());
    }
    let mut why = format!("no trusted issuer for {name}");
    for issuer in anchors.iter().chain(pool) {
        if issuer.subject().as_raw() != cert.issuer().as_raw() || issuer.as_raw() == cert.as_raw() {
            continue;
        }
        match issues(issuer, cert, below).and_then(|()| reaches(issuer, pool, anchors, below + 1)) {
            Ok(()) => return Ok(()),
            Err(error) => why = error,
        }
    }
    Err(why)
}

/// The most CAs between a signer and its root.
const MAX_DEPTH: u32 = 8;

/// Whether `issuer` may sign certificates and signed `cert`.
fn issues(issuer: &X509Certificate, cert: &X509Certificate, below: u32) -> Result<(), String> {
    let name = issuer.subject();
    let basic = issuer.basic_constraints().map_err(|error| error.to_string())?;
    let Some(basic) = basic.filter(|basic| basic.value.ca) else {
        return Err(format!("{name} is not a CA"));
    };
    if basic.value.path_len_constraint.is_some_and(|max| below > max) {
        return Err(format!("{name} allows a shorter chain"));
    }
    if issuer.key_usage().map_err(|error| error.to_string())?.is_some_and(|usage| !usage.value.key_cert_sign()) {
        return Err(format!("{name} may not sign certificates"));
    }
    cert.verify_signature(Some(issuer.public_key()))
        .map_err(|error| format!("{}'s signature does not verify: {error}", cert.subject()))
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
