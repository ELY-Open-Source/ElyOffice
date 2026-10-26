//! CryptoAPI: the chain engine and the base chain policy.

use std::{io, ptr};

use windows_sys::Win32::Security::Cryptography::{
    CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL, CERT_CHAIN_CONTEXT, CERT_CHAIN_DISABLE_AUTH_ROOT_AUTO_UPDATE,
    CERT_CHAIN_ENGINE_CONFIG, CERT_CHAIN_PARA, CERT_CHAIN_POLICY_BASE, CERT_CHAIN_POLICY_PARA,
    CERT_CHAIN_POLICY_STATUS, CERT_CONTEXT, CERT_STORE_ADD_ALWAYS, CERT_STORE_PROV_MEMORY,
    CertAddEncodedCertificateToStore, CertCloseStore, CertCreateCertificateChainEngine,
    CertCreateCertificateContext, CertFreeCertificateChain, CertFreeCertificateChainEngine,
    CertFreeCertificateContext, CertGetCertificateChain, CertOpenStore, CertVerifyCertificateChainPolicy,
    HCERTCHAINENGINE, HCERTSTORE, PKCS_7_ASN_ENCODING, X509_ASN_ENCODING,
};

use super::Trust;

const ENCODING: u32 = X509_ASN_ENCODING | PKCS_7_ASN_ENCODING;

/// `anchors` replaces the system's roots; tests only.
pub fn check(chain: &[Vec<u8>], anchors: Option<&[Vec<u8>]>) -> io::Result<Trust> {
    let Some((leaf, rest)) = chain.split_first() else {
        return Err(io::Error::other("a signature with no certificate"));
    };
    let leaf = Cert::new(leaf)?;
    let others = Store::holding(rest)?;
    let engine = anchors.map(|anchors| Engine::rooted(Store::holding(anchors)?)).transpose()?;
    let para = CERT_CHAIN_PARA { cbSize: size_of::<CERT_CHAIN_PARA>() as u32, ..Default::default() };
    // No network: revocation is never checked.
    let flags = CERT_CHAIN_CACHE_ONLY_URL_RETRIEVAL | CERT_CHAIN_DISABLE_AUTH_ROOT_AUTO_UPDATE;
    let mut built = ptr::null_mut();
    let engine_handle = engine.as_ref().map_or(ptr::null_mut(), |engine| engine.handle);
    if unsafe {
        CertGetCertificateChain(engine_handle, leaf.0, ptr::null(), others.0, &para, flags, ptr::null(), &mut built)
    } == 0
    {
        return Err(io::Error::last_os_error());
    }
    let built = Chain(built);
    let policy = CERT_CHAIN_POLICY_PARA { cbSize: size_of::<CERT_CHAIN_POLICY_PARA>() as u32, ..Default::default() };
    let mut status =
        CERT_CHAIN_POLICY_STATUS { cbSize: size_of::<CERT_CHAIN_POLICY_STATUS>() as u32, ..Default::default() };
    if unsafe { CertVerifyCertificateChainPolicy(CERT_CHAIN_POLICY_BASE, built.0, &policy, &mut status) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(match status.dwError {
        0 => Trust::Trusted,
        code => Trust::Untrusted(io::Error::from_raw_os_error(code as i32).to_string()),
    })
}

struct Cert(*const CERT_CONTEXT);

impl Cert {
    fn new(der: &[u8]) -> io::Result<Self> {
        let context = unsafe { CertCreateCertificateContext(ENCODING, der.as_ptr(), der.len() as u32) };
        match context.is_null() {
            true => Err(io::Error::last_os_error()),
            false => Ok(Self(context)),
        }
    }
}

impl Drop for Cert {
    fn drop(&mut self) {
        unsafe { CertFreeCertificateContext(self.0) };
    }
}

struct Store(HCERTSTORE);

impl Store {
    /// A memory store holding `ders`.
    fn holding(ders: &[Vec<u8>]) -> io::Result<Self> {
        let store = unsafe { CertOpenStore(CERT_STORE_PROV_MEMORY, 0, 0, 0, ptr::null()) };
        if store.is_null() {
            return Err(io::Error::last_os_error());
        }
        let store = Self(store);
        for der in ders {
            let added = unsafe {
                CertAddEncodedCertificateToStore(
                    store.0,
                    ENCODING,
                    der.as_ptr(),
                    der.len() as u32,
                    CERT_STORE_ADD_ALWAYS,
                    ptr::null_mut(),
                )
            };
            if added == 0 {
                return Err(io::Error::last_os_error());
            }
        }
        Ok(store)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        unsafe { CertCloseStore(self.0, 0) };
    }
}

struct Engine {
    handle: HCERTCHAINENGINE,
    _roots: Store,
}

impl Engine {
    /// An engine whose only roots are in `roots`.
    fn rooted(roots: Store) -> io::Result<Self> {
        let config = CERT_CHAIN_ENGINE_CONFIG {
            cbSize: size_of::<CERT_CHAIN_ENGINE_CONFIG>() as u32,
            hExclusiveRoot: roots.0,
            ..Default::default()
        };
        let mut handle = ptr::null_mut();
        if unsafe { CertCreateCertificateChainEngine(&config, &mut handle) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { handle, _roots: roots })
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe { CertFreeCertificateChainEngine(self.handle) };
    }
}

struct Chain(*mut CERT_CHAIN_CONTEXT);

impl Drop for Chain {
    fn drop(&mut self) {
        unsafe { CertFreeCertificateChain(self.0) };
    }
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
