mod oauth_fake;

use std::{
    thread,
    time::{Duration, SystemTime},
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ely_mail_engine::{
    Error,
    oauth::{SignIn, refresh},
};
use oauth_fake::{browse, endpoint, id_token, provider, query, signed_in};
use sha2::{Digest, Sha256};

#[tokio::test]
async fn sign_in_exchanges_the_code_with_pkce() {
    let body = format!(
        r#"{{"access_token":"at-1","expires_in":3600,"refresh_token":"rt-1","id_token":"{}"}}"#,
        id_token(r#"{"email":"ana@example.com"}"#)
    );
    let (token, forms) = endpoint(200, body);
    let sign_in = SignIn::start(provider(token), "ana@example.com")
        .await
        .unwrap();
    let asked = query(sign_in.url());
    assert!(sign_in.url().starts_with("https://auth.example/authorize?"));
    assert_eq!(asked["client_id"], "client-1");
    assert_eq!(asked["response_type"], "code");
    assert_eq!(asked["code_challenge_method"], "S256");
    assert_eq!(asked["login_hint"], "ana@example.com");
    assert_eq!(asked["scope"], "mail openid email");
    assert_eq!(asked["prompt"], "consent");
    let redirect = asked["redirect_uri"].clone();
    assert!(redirect.starts_with("http://127.0.0.1:"));
    let state = asked["state"].clone();
    let browser = thread::spawn(move || {
        let ignored = browse(&redirect, "/favicon.ico");
        let done = browse(&redirect, &format!("/?code=c%2B1&state={state}"));
        (ignored, done, redirect)
    });
    let before = SystemTime::now();
    let tokens = sign_in.finish().await.unwrap();
    let (ignored, done, redirect) = browser.join().unwrap();
    assert_eq!(ignored, "HTTP/1.1 404 Not Found");
    assert_eq!(done, "HTTP/1.1 200 OK");
    assert_eq!(tokens.access, "at-1");
    assert_eq!(tokens.refresh.as_deref(), Some("rt-1"));
    assert_eq!(tokens.email.as_deref(), Some("ana@example.com"));
    let left = tokens.expires_at.duration_since(before).unwrap();
    assert!(left >= Duration::from_secs(3600) && left < Duration::from_secs(3610));
    let form = forms.recv().unwrap();
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["code"], "c+1");
    assert!(!form.contains_key("client_secret"));
    assert_eq!(form["redirect_uri"], redirect);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(form["code_verifier"].as_bytes()));
    assert_eq!(challenge, asked["code_challenge"]);
}

#[tokio::test]
async fn a_wrong_state_is_refused() {
    let sign_in = SignIn::start(provider("http://127.0.0.1:9/token".into()), "")
        .await
        .unwrap();
    let asked = query(sign_in.url());
    assert!(!asked.contains_key("login_hint"));
    let redirect = asked["redirect_uri"].clone();
    let browser = thread::spawn(move || browse(&redirect, "/?code=x&state=forged"));
    let err = sign_in.finish().await.err().unwrap();
    browser.join().unwrap();
    assert!(matches!(err, Error::SignIn(text) if text.contains("state")));
}

#[tokio::test]
async fn a_declined_consent_reports_the_reason() {
    let sign_in = SignIn::start(provider("http://127.0.0.1:9/token".into()), "")
        .await
        .unwrap();
    let asked = query(sign_in.url());
    let (redirect, state) = (asked["redirect_uri"].clone(), asked["state"].clone());
    let path = format!("/?error=access_denied&error_description=user+said+no&state={state}");
    let browser = thread::spawn(move || browse(&redirect, &path));
    let err = sign_in.finish().await.err().unwrap();
    browser.join().unwrap();
    assert!(matches!(err, Error::SignIn(text) if text == "access_denied: user said no"));
}

#[tokio::test]
async fn a_sign_in_without_a_refresh_token_fails() {
    let body = r#"{"access_token":"at-1","expires_in":3600}"#.to_owned();
    let err = signed_in(body).await.err().unwrap();
    assert!(matches!(err, Error::SignIn(text) if text.contains("refresh token")));
}

#[tokio::test]
async fn the_username_stands_in_for_a_missing_email() {
    let body = format!(
        r#"{{"access_token":"at-1","expires_in":3600,"refresh_token":"rt-1","id_token":"{}"}}"#,
        id_token(r#"{"preferred_username":"bo@outlook.com"}"#)
    );
    let tokens = signed_in(body).await.unwrap();
    assert_eq!(tokens.email.as_deref(), Some("bo@outlook.com"));
}

#[tokio::test]
async fn refresh_rotates_the_token() {
    let body = r#"{"access_token":"at-2","expires_in":60,"refresh_token":"rt-2"}"#.to_owned();
    let (token, forms) = endpoint(200, body);
    let tokens = refresh(&provider(token), "rt-1").await.unwrap();
    assert_eq!(
        (tokens.access.as_str(), tokens.refresh.as_deref()),
        ("at-2", Some("rt-2"))
    );
    assert_eq!(tokens.email, None);
    let form = forms.recv().unwrap();
    assert_eq!(form["grant_type"], "refresh_token");
    assert_eq!(form["refresh_token"], "rt-1");
    assert!(!form.contains_key("client_secret"));
}

#[tokio::test]
async fn a_revoked_refresh_token_needs_sign_in_again() {
    for error in ["invalid_grant", "interaction_required"] {
        let body = format!(r#"{{"error":"{error}","error_description":"revoked"}}"#);
        let (token, _) = endpoint(400, body);
        let err = refresh(&provider(token), "rt-1").await.err().unwrap();
        assert!(
            matches!(&err, Error::SignInAgain(text) if text == &format!("{error}: revoked")),
            "{err}"
        );
    }
}

#[tokio::test]
async fn other_refusals_stay_sign_in_errors() {
    let (token, _) = endpoint(401, r#"{"error":"invalid_client"}"#.to_owned());
    let err = refresh(&provider(token), "rt-1").await.err().unwrap();
    assert!(matches!(err, Error::SignIn(text) if text == "invalid_client"));
    let (token, _) = endpoint(502, "<html>bad gateway</html>".to_owned());
    let err = refresh(&provider(token), "rt-1").await.err().unwrap();
    assert!(matches!(err, Error::SignIn(text) if text.contains("502")));
}
