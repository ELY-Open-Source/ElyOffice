//! Remote pictures, fetched only once the reader allows.

use std::time::Duration;

use ureq::Agent;

use crate::{Error, Result};

/// Larger pictures are refused.
pub const LARGEST: u64 = 10 << 20;
const TIMEOUT: Duration = Duration::from_secs(20);

/// A web picture's type and bytes; blocking.
pub fn picture(url: &str) -> Result<(String, Vec<u8>)> {
    let lower = url.to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(Error::Fetch("only web pictures are fetched".into()));
    }
    let agent: Agent = Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .proxy(crate::proxy::for_web()?)
        .build()
        .into();
    let mut response = agent
        .get(url)
        .call()
        .map_err(|err| Error::Fetch(err.to_string()))?;
    let kind = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !kind.starts_with("image/") {
        return Err(Error::Fetch(format!("{kind:?} is not a picture")));
    }
    let bytes = response
        .body_mut()
        .with_config()
        .limit(LARGEST)
        .read_to_vec()
        .map_err(|err| Error::Fetch(err.to_string()))?;
    Ok((kind, bytes))
}
