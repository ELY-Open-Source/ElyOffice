use std::time::SystemTime;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;

use crate::{Error, Result};

mod loopback;
mod token;

/// One OAuth authorization server and this app's client.
pub struct Provider {
    pub name: &'static str,
    pub authorize: String,
    pub token: String,
    pub client_id: String,
    pub scope: String,
    /// Authorize parameters this server adds.
    pub extra: &'static [(&'static str, &'static str)],
    /// The registered loopback host.
    pub redirect_host: &'static str,
}

/// Tokens from one exchange or refresh.
pub struct Tokens {
    pub access: String,
    pub expires_at: SystemTime,
    /// A new refresh token; store it, drop the old one.
    pub refresh: Option<String>,
    /// The signed-in address, from the ID token.
    pub email: Option<String>,
}

/// Outlook.com and Microsoft 365.
pub fn microsoft() -> Result<Provider> {
    Ok(outlook(built(
        "microsoft",
        option_env!("ELY_MICROSOFT_CLIENT_ID"),
    )?))
}

fn outlook(client_id: String) -> Provider {
    Provider {
        name: "microsoft",
        authorize: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize".into(),
        token: "https://login.microsoftonline.com/common/oauth2/v2.0/token".into(),
        client_id,
        scope: [
            "https://outlook.office.com/IMAP.AccessAsUser.All",
            "https://outlook.office.com/POP.AccessAsUser.All",
            "https://outlook.office.com/SMTP.Send",
            "offline_access openid email profile",
        ]
        .join(" "),
        // Never reuse the browser's account silently.
        extra: &[("prompt", "select_account")],
        // Port ignored only for localhost.
        redirect_host: "localhost",
    }
}

fn built(name: &'static str, value: Option<&str>) -> Result<String> {
    match value {
        Some(value) if !value.is_empty() => Ok(value.to_owned()),
        _ => Err(Error::Unavailable(name)),
    }
}

/// A sign-in waiting for the browser to come back.
pub struct SignIn {
    provider: Provider,
    listener: TcpListener,
    url: String,
    redirect: String,
    state: String,
    verifier: String,
}

impl SignIn {
    /// Opens the loopback port and builds the browser URL.
    pub async fn start(provider: Provider, hint: &str) -> Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .map_err(|err| Error::SignIn(format!("no loopback port: {err}")))?;
        let port = listener
            .local_addr()
            .map_err(|err| Error::SignIn(format!("no loopback port: {err}")))?
            .port();
        let redirect = format!("http://{}:{port}", provider.redirect_host);
        let state = random();
        let verifier = random();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut query = vec![
            ("client_id", provider.client_id.as_str()),
            ("response_type", "code"),
            ("redirect_uri", redirect.as_str()),
            ("scope", provider.scope.as_str()),
            ("state", state.as_str()),
            ("code_challenge", challenge.as_str()),
            ("code_challenge_method", "S256"),
        ];
        query.extend_from_slice(provider.extra);
        if !hint.is_empty() {
            query.push(("login_hint", hint));
        }
        let url = format!("{}?{}", provider.authorize, encode(&query));
        log::info!("oauth: {} sign-in waits on 127.0.0.1:{port}", provider.name);
        Ok(Self {
            provider,
            listener,
            url,
            redirect,
            state,
            verifier,
        })
    }

    /// The page to open in the system browser.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Waits for the redirect, then trades the code for tokens.
    pub async fn finish(self) -> Result<Tokens> {
        let code = loopback::code(&self.listener, &self.state).await?;
        log::info!("oauth: {} returned a code", self.provider.name);
        drop(self.listener);
        let form = vec![
            ("grant_type", "authorization_code".to_owned()),
            ("client_id", self.provider.client_id.clone()),
            ("code", code),
            ("code_verifier", self.verifier),
            ("redirect_uri", self.redirect),
        ];
        let tokens = token::post(&self.provider, form, false).await?;
        if tokens.refresh.is_none() {
            return Err(Error::SignIn("no refresh token granted".into()));
        }
        Ok(tokens)
    }
}

/// A fresh access token; a refused refresh needs sign-in again.
pub async fn refresh(provider: &Provider, refresh: &str) -> Result<Tokens> {
    let form = vec![
        ("grant_type", "refresh_token".to_owned()),
        ("client_id", provider.client_id.clone()),
        ("refresh_token", refresh.to_owned()),
    ];
    token::post(provider, form, true).await
}

/// 32 random bytes, base64url: a PKCE verifier or a state.
fn random() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("system randomness");
    URL_SAFE_NO_PAD.encode(bytes)
}

fn encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(key, value)| format!("{}={}", escape(key), escape(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn escape(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_client_id_makes_the_provider_unavailable() {
        assert!(matches!(
            built("microsoft", None),
            Err(Error::Unavailable("microsoft"))
        ));
        assert!(matches!(
            built("microsoft", Some("")),
            Err(Error::Unavailable("microsoft"))
        ));
        assert_eq!(built("microsoft", Some("id-1")).unwrap(), "id-1");
    }

    async fn asked(provider: Provider) -> Vec<(String, String)> {
        let sign_in = SignIn::start(provider, "").await.unwrap();
        let (_, query) = sign_in.url().split_once('?').unwrap();
        form_urlencoded::parse(query.as_bytes())
            .into_owned()
            .collect()
    }

    fn value<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
        pairs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    #[tokio::test]
    async fn outlook_asks_for_offline_access_and_the_address_claims() {
        let pairs = asked(outlook("id".into())).await;
        let scope: Vec<&str> = value(&pairs, "scope").unwrap().split(' ').collect();
        for needed in ["offline_access", "openid", "email", "profile"] {
            assert!(scope.contains(&needed), "{needed}");
        }
        assert_eq!(value(&pairs, "access_type"), None);
        assert_eq!(value(&pairs, "prompt"), Some("select_account"));
        assert!(
            value(&pairs, "redirect_uri")
                .unwrap()
                .starts_with("http://localhost:")
        );
    }

    #[test]
    fn query_values_are_escaped() {
        assert_eq!(
            encode(&[("scope", "a b/c"), ("x", "é~")]),
            "scope=a%20b%2Fc&x=%C3%A9~"
        );
    }
}
