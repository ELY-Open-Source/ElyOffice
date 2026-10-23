use std::time::{Duration, SystemTime};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::Deserialize;
use ureq::Agent;

use super::{Provider, Tokens};
use crate::{Error, Result};

const TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Deserialize)]
struct Granted {
    access_token: String,
    expires_in: u64,
    refresh_token: Option<String>,
    id_token: Option<String>,
}

#[derive(Deserialize)]
struct Refused {
    error: String,
    error_description: Option<String>,
}

#[derive(Deserialize)]
struct Claims {
    email: Option<String>,
    preferred_username: Option<String>,
}

/// Posts to the token endpoint off the async thread.
pub(super) async fn post(
    provider: &Provider,
    form: Vec<(&'static str, String)>,
    refreshing: bool,
) -> Result<Tokens> {
    let url = provider.token.clone();
    let name = provider.name;
    let started = SystemTime::now();
    let (status, body) = tokio::task::spawn_blocking(move || send(&url, &form))
        .await
        .map_err(|err| Error::SignIn(format!("token request stopped: {err}")))??;
    if status == 200 {
        let tokens = granted(&body, started)?;
        log::info!(
            "oauth: {name} token granted, refresh {}",
            tokens.refresh.is_some()
        );
        return Ok(tokens);
    }
    let refused: Refused = serde_json::from_str(&body)
        .map_err(|_| Error::SignIn(format!("token endpoint answered {status}")))?;
    let reason = match refused.error_description {
        Some(text) => format!("{}: {text}", refused.error),
        None => refused.error.clone(),
    };
    log::warn!("oauth: {name} token refused ({status}, {})", refused.error);
    if refreshing
        && matches!(
            refused.error.as_str(),
            "invalid_grant" | "interaction_required"
        )
    {
        return Err(Error::SignInAgain(reason));
    }
    Err(Error::SignIn(reason))
}

fn send(url: &str, form: &[(&'static str, String)]) -> Result<(u16, String)> {
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .proxy(crate::proxy::for_web()?)
        .http_status_as_error(false)
        .build()
        .into();
    let mut response = agent
        .post(url)
        .send_form(form.iter().map(|(key, value)| (*key, value.as_str())))
        .map_err(|err| Error::SignIn(format!("token endpoint unreachable: {err}")))?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|err| Error::SignIn(format!("token reply unreadable: {err}")))?;
    Ok((status, body))
}

fn granted(body: &str, started: SystemTime) -> Result<Tokens> {
    let granted: Granted = serde_json::from_str(body)
        .map_err(|err| Error::SignIn(format!("token reply unreadable: {err}")))?;
    let email = match &granted.id_token {
        Some(token) => email(token)?,
        None => None,
    };
    Ok(Tokens {
        access: granted.access_token,
        expires_at: started + Duration::from_secs(granted.expires_in),
        refresh: granted.refresh_token,
        email,
    })
}

/// The address claim; the token came straight from the endpoint.
fn email(token: &str) -> Result<Option<String>> {
    let unreadable = || Error::SignIn("ID token unreadable".into());
    let payload = token.split('.').nth(1).ok_or_else(unreadable)?;
    let bytes = URL_SAFE_NO_PAD.decode(payload).map_err(|_| unreadable())?;
    let claims: Claims = serde_json::from_slice(&bytes).map_err(|_| unreadable())?;
    Ok(claims.email.or(claims.preferred_username))
}
