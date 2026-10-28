//! Whether this system trusts a certificate chain (PDF-12).

use std::io;

/// The system's answer for a chain, leaf first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trust {
    Trusted,
    /// The system's own words for why not.
    Untrusted(String),
}

/// Evaluates `chain` (DER, leaf first) against the system's trust settings.
pub fn check(chain: &[Vec<u8>]) -> io::Result<Trust> {
    #[cfg(target_os = "macos")]
    let checked = mac::check(chain, None);
    #[cfg(windows)]
    let checked = windows::check(chain, None);
    #[cfg(target_os = "linux")]
    let checked = linux::check(chain, None);
    checked
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(test)]
mod tests {
    /// The certificate in tests/fixtures/pdf/signer.p12, self-signed.
    pub(super) fn signer() -> Vec<u8> {
        include_bytes!("../../../tests/fixtures/pdf/signer.der").to_vec()
    }

    /// A test root; `leaf` chains to it.
    pub(super) fn root() -> Vec<u8> {
        include_bytes!("../../../tests/fixtures/pdf/chain-root.der").to_vec()
    }

    pub(super) fn leaf() -> Vec<u8> {
        include_bytes!("../../../tests/fixtures/pdf/chain-leaf.der").to_vec()
    }

    #[cfg(target_os = "linux")]
    use super::linux::check;
    #[cfg(target_os = "macos")]
    use super::mac::check;
    #[cfg(windows)]
    use super::windows::check;
    use super::Trust;

    /// A test certificate, signed by `by` or itself.
    struct Made {
        der: Vec<u8>,
        params: rcgen::CertificateParams,
        key: rcgen::KeyPair,
    }

    fn made(name: impl Into<rcgen::DnValue>, (from, to): (i32, i32), ca: rcgen::IsCa, by: Option<&Made>, extra: Vec<rcgen::CustomExtension>) -> Made {
        use rcgen::{CertificateParams, DnType, Issuer, KeyPair, date_time_ymd};
        let mut params = CertificateParams::new(Vec::<String>::new()).expect("params");
        params.distinguished_name.push(DnType::CommonName, name);
        params.not_before = date_time_ymd(from, 1, 1);
        params.not_after = date_time_ymd(to, 1, 1);
        params.is_ca = ca;
        params.custom_extensions = extra;
        params.use_authority_key_identifier_extension = true;
        let key = KeyPair::generate().expect("a key");
        let cert = match by {
            Some(by) => params.signed_by(&key, &Issuer::from_params(&by.params, &by.key)),
            None => params.self_signed(&key),
        };
        Made { der: cert.expect("signs").der().to_vec(), params, key }
    }

    const NOW: (i32, i32) = (2020, 2050);

    fn ca() -> rcgen::IsCa {
        rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained)
    }

    fn untrusted(chain: &[Vec<u8>], anchor: &Made) -> bool {
        matches!(check(chain, Some(std::slice::from_ref(&anchor.der))).expect("checks"), Trust::Untrusted(_))
    }

    #[test]
    fn a_root_signs_only_while_it_is_valid() {
        let root = made("Ely test root", NOW, ca(), None, vec![]);
        assert!(!untrusted(std::slice::from_ref(&root.der), &root));
        for span in [(2020, 2021), (2040, 2050)] {
            let root = made("Ely test root", span, ca(), None, vec![]);
            assert!(untrusted(std::slice::from_ref(&root.der), &root), "{span:?}");
        }
    }

    #[test]
    fn a_ca_signer_chains_like_any_other() {
        for signer in [rcgen::IsCa::ExplicitNoCa, ca()] {
            let root = made("Ely test root", NOW, ca(), None, vec![]);
            let child = made("Ely test signer", NOW, signer, Some(&root), vec![]);
            assert!(!untrusted(std::slice::from_ref(&child.der), &root), "{signer:?}");
            let other = made("Ely test root", NOW, ca(), None, vec![]);
            assert!(untrusted(std::slice::from_ref(&child.der), &other), "{signer:?} under a namesake");
        }
    }

    #[test]
    fn a_ca_signer_needs_a_ca_between() {
        let root = made("Ely test root", NOW, ca(), None, vec![]);
        for (between, trusted) in [(ca(), true), (rcgen::IsCa::ExplicitNoCa, false)] {
            let middle = made("Ely test middle", NOW, between, Some(&root), vec![]);
            let signer = made("Ely test signer", NOW, ca(), Some(&middle), vec![]);
            assert_eq!(!untrusted(&[signer.der, middle.der], &root), trusted, "{between:?}");
        }
    }

    #[test]
    fn a_critical_extension_left_unchecked_breaks_the_chain() {
        let root = made("Ely test root", NOW, ca(), None, vec![]);
        let cases: [(&[u64], Vec<u8>); 2] = [(&[2, 5, 29, 36], vec![0x30, 3, 0x80, 1, 0]), (&[2, 5, 29, 37], vec![5, 0])];
        for (oid, content) in cases {
            let mut ext = rcgen::CustomExtension::from_oid_content(oid, content);
            ext.set_criticality(true);
            let middle = made("Ely test middle", NOW, ca(), Some(&root), vec![ext]);
            let signer = made("Ely test signer", NOW, ca(), Some(&middle), vec![]);
            assert!(untrusted(&[signer.der, middle.der], &root), "{oid:?}");
        }
    }

    #[test]
    fn a_rollover_ca_is_not_counted_in_the_path_length() {
        let root = made("Ely test root", NOW, rcgen::IsCa::Ca(rcgen::BasicConstraints::Constrained(0)), None, vec![]);
        for (name, trusted) in [("Ely test root", true), ("Ely test middle", false)] {
            let middle = made(name, NOW, ca(), Some(&root), vec![]);
            let signer = made("Ely test signer", NOW, ca(), Some(&middle), vec![]);
            assert_eq!(!untrusted(&[signer.der, middle.der], &root), trusted, "{name}");
        }
    }

    /// Windows compares names byte for byte; RFC 5280 does not.
    #[cfg(target_os = "linux")]
    #[test]
    fn a_rollover_ca_keeps_its_name_in_another_encoding() {
        let root = made("Ely test root", NOW, rcgen::IsCa::Ca(rcgen::BasicConstraints::Constrained(0)), None, vec![]);
        let printable = rcgen::string::PrintableString::try_from("Ely test root").expect("printable");
        let middle = made(rcgen::DnValue::PrintableString(printable), NOW, ca(), Some(&root), vec![]);
        let signer = made("Ely test signer", NOW, ca(), Some(&middle), vec![]);
        assert!(!untrusted(&[signer.der, middle.der], &root));
    }
}

#[cfg(target_os = "macos")]
mod mac {
    use std::io;

    use security_framework::{certificate::SecCertificate, policy::SecPolicy, trust::SecTrust};

    use super::Trust;

    /// `anchors` replaces the system's roots; tests only.
    pub fn check(chain: &[Vec<u8>], anchors: Option<&[Vec<u8>]>) -> io::Result<Trust> {
        let certs = |ders: &[Vec<u8>]| -> io::Result<Vec<SecCertificate>> {
            ders.iter()
                .map(|der| SecCertificate::from_der(der).map_err(io::Error::other))
                .collect()
        };
        let leaf = certs(chain)?;
        if leaf.is_empty() {
            return Err(io::Error::other("a signature with no certificate"));
        }
        let mut trust = SecTrust::create_with_certificates(&leaf, &[SecPolicy::create_x509()])
            .map_err(io::Error::other)?;
        // No network: revocation is never checked.
        trust
            .set_network_fetch_allowed(false)
            .map_err(io::Error::other)?;
        if let Some(anchors) = anchors {
            trust
                .set_anchor_certificates(&certs(anchors)?)
                .map_err(io::Error::other)?;
        }
        Ok(match trust.evaluate_with_error() {
            Ok(()) => Trust::Trusted,
            Err(why) => Trust::Untrusted(why.description().to_string()),
        })
    }

    #[cfg(test)]
    mod tests {
        use super::super::tests::{leaf, root, signer};
        use super::{Trust, check};

        #[test]
        fn a_self_signed_signer_is_not_trusted_by_the_system() {
            assert!(
                matches!(check(&[signer()], None).expect("checks"), Trust::Untrusted(why) if !why.is_empty())
            );
        }

        #[test]
        fn a_chain_to_a_trusted_root_is_trusted() {
            assert_eq!(check(&[leaf()], Some(&[root()])).expect("checks"), Trust::Trusted);
            assert_eq!(
                check(&[signer()], Some(&[signer()])).expect("checks"),
                Trust::Trusted
            );
        }

        #[test]
        fn a_chain_to_another_root_is_not() {
            assert!(matches!(check(&[leaf()], Some(&[signer()])).expect("checks"), Trust::Untrusted(_)));
        }
    }
}
