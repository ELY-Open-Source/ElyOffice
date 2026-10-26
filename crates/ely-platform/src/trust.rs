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
