//! The system's CA store, by webpki or the base policy.

use std::io;

use rustls_pki_types::{CertificateDer, UnixTime};
use unicode_normalization::UnicodeNormalization as _;
use webpki::{ALL_VERIFICATION_ALGS, EndEntityCert, ExtendedKeyUsageValidator, KeyPurposeIdIter, anchor_from_trusted_cert};
use x509_parser::prelude::{
    AttributeTypeAndValue, ParsedExtension, X509Certificate, X509Extension, X509Name, parse_x509_certificate,
};

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
        reaches(&signer, &pool, &anchors, 0, 0)
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

/// Whether `cert` chains to an anchor; `below` counts path-length CAs.
fn reaches(
    cert: &X509Certificate,
    pool: &[X509Certificate],
    anchors: &[X509Certificate],
    below: u32,
    depth: u32,
) -> Result<(), String> {
    let name = cert.subject();
    if !cert.validity().is_valid() {
        return Err(format!("{name} is outside its validity period"));
    }
    for ext in cert.extensions() {
        enforced(ext).map_err(|why| format!("{name} {why}"))?;
    }
    // Fails closed: name constraints are not evaluated.
    if cert.name_constraints().map_err(|error| error.to_string())?.is_some() {
        return Err(format!("{name} has name constraints"));
    }
    if anchors.iter().any(|anchor| anchor.as_raw() == cert.as_raw()) {
        return Ok(());
    }
    if depth > MAX_DEPTH {
        return Err("the chain is too long".into());
    }
    let mut why = format!("no trusted issuer for {name}");
    for issuer in anchors.iter().chain(pool) {
        if !same_name(issuer.subject(), cert.issuer()) || issuer.as_raw() == cert.as_raw() {
            continue;
        }
        // Self-issued CAs do not count (RFC 5280, 4.2.1.9).
        let counted = below + u32::from(!same_name(issuer.subject(), issuer.issuer()));
        match issues(issuer, cert, below).and_then(|()| reaches(issuer, pool, anchors, counted, depth + 1)) {
            Ok(()) => return Ok(()),
            Err(error) => why = error,
        }
    }
    Err(why)
}

/// An attribute's type; its folded value or tagged bytes.
type Attr = (String, Result<String, (u32, Vec<u8>)>);

/// Names equal by RFC 5280 7.1 and RFC 4518.
fn same_name(a: &X509Name, b: &X509Name) -> bool {
    let rdns = |name: &X509Name| -> Vec<Vec<Attr>> {
        name.iter()
            .map(|rdn| {
                let mut attrs: Vec<_> = rdn.iter().map(|attr| (attr.attr_type().to_id_string(), folded(attr))).collect();
                attrs.sort();
                attrs
            })
            .collect()
    };
    a.as_raw() == b.as_raw() || rdns(a) == rdns(b)
}

/// A string prepared by RFC 4518; else tag and bytes.
fn folded(attr: &AttributeTypeAndValue) -> Result<String, (u32, Vec<u8>)> {
    let value = attr.attr_value();
    let (tag, bytes) = (value.tag().0, attr.as_slice());
    // RFC 4518 2.1: each string type to Unicode first.
    let text = match tag {
        BMP => wide(bytes, 2).and_then(|units| char::decode_utf16(units.map(|unit| unit as u16)).collect::<Result<String, _>>().ok()),
        UNIVERSAL => wide(bytes, 4).and_then(|units| units.map(char::from_u32).collect::<Option<String>>()),
        _ => attr.as_str().ok().map(str::to_string),
    };
    text.as_deref().and_then(prepared).ok_or_else(|| (tag, bytes.to_vec()))
}

/// The ASN.1 tags of BMPString and UniversalString.
const BMP: u32 = 30;
const UNIVERSAL: u32 = 28;

/// Big-endian units of `size` bytes; None on a ragged end.
fn wide(bytes: &[u8], size: usize) -> Option<impl Iterator<Item = u32> + '_> {
    bytes.len().is_multiple_of(size).then(|| bytes.chunks(size).map(|unit| unit.iter().fold(0, |sum, byte| sum << 8 | u32::from(*byte))))
}

/// RFC 4518, 2.2 to 2.6, for case-ignore matching.
fn prepared(text: &str) -> Option<String> {
    use stringprep::tables;
    // Unicode 3.2's list: a later NFKC would map some away.
    if text.chars().any(tables::unassigned_code_point) {
        return None;
    }
    let mapped: String = text
        .chars()
        .filter(|&c| !mapped_to_nothing(c))
        .map(|c| match c {
            '\u{9}'..='\u{D}' | '\u{85}' => ' ',
            _ if c.is_whitespace() => ' ',
            _ => c,
        })
        .flat_map(tables::case_fold_for_nfkc)
        .collect();
    let normal: String = mapped.nfkc().collect();
    let prohibited = |c: char| {
        tables::unassigned_code_point(c)
            || tables::change_display_properties_or_deprecated(c)
            || tables::private_use(c)
            || tables::non_character_code_point(c)
            || tables::surrogate_code(c)
            || c == '\u{FFFD}'
    };
    if normal.chars().any(prohibited) {
        return None;
    }
    Some(spaced(&normal))
}

/// RFC 4518 2.6.1: spaces before a mark count.
fn spaced(text: &str) -> String {
    let (mut words, mut word) = (Vec::new(), String::new());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        let marked = chars.peek().is_some_and(|&next| unicode_normalization::char::is_combining_mark(next));
        if c != ' ' || marked {
            word.push(c);
        } else if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words.join(" ")
}

/// RFC 4518, 2.2: deleted, controls included.
fn mapped_to_nothing(c: char) -> bool {
    matches!(c,
        '\u{AD}' | '\u{1806}' | '\u{34F}' | '\u{180B}'..='\u{180D}' | '\u{FE00}'..='\u{FE0F}' | '\u{FFFC}' | '\u{200B}'
        | '\u{0}'..='\u{8}' | '\u{E}'..='\u{1F}' | '\u{7F}'..='\u{84}' | '\u{86}'..='\u{9F}' | '\u{6DD}' | '\u{70F}'
        | '\u{180E}' | '\u{200C}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{2063}'
        | '\u{206A}'..='\u{206F}' | '\u{FEFF}' | '\u{FFF9}'..='\u{FFFB}' | '\u{1D173}'..='\u{1D17A}' | '\u{E0001}'
        | '\u{E0020}'..='\u{E007F}')
}

/// Refuses an unreadable extension, or a critical one left unchecked.
fn enforced(ext: &X509Extension) -> Result<(), String> {
    match ext.parsed_extension() {
        ParsedExtension::ParseError { error } => Err(format!("has an unreadable extension {}: {error}", ext.oid)),
        // Any usage is accepted, as on macOS and Windows.
        ParsedExtension::BasicConstraints(_) | ParsedExtension::KeyUsage(_) | ParsedExtension::ExtendedKeyUsage(_) => Ok(()),
        _ if ext.critical => Err(format!("has a critical extension Ely does not check: {}", ext.oid)),
        _ => Ok(()),
    }
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
