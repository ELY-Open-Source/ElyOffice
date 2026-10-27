use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Write},
    net::IpAddr,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use ureq::{
    Agent, Body,
    http::{Response, Uri, header::LOCATION},
    unversioned::{
        resolver::DefaultResolver,
        transport::{
            Buffers, ConnectionDetails, Connector, DefaultConnector, NextTimeout, Transport,
        },
    },
};

const MANIFEST_LIMIT: u64 = 1 << 20;
const DOWNLOAD_LIMIT: u64 = 4 << 30;
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_REDIRECTS: usize = 5;

/// Longest a download may go without a byte.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq)]
pub struct Release {
    pub version: Version,
    pub url: String,
    pub sha256: String,
}

#[derive(Debug, PartialEq)]
pub enum Check {
    Current,
    Available(Release),
}

#[derive(Deserialize)]
struct Manifest {
    version: Version,
    assets: HashMap<String, Asset>,
}

#[derive(Deserialize)]
struct Asset {
    url: String,
    sha256: String,
}

/// The manifest key of this build, such as `macos-aarch64`.
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Fetches the manifest and compares it with `current`. Never installs.
pub fn check(current: &Version, manifest_url: &str) -> Result<Check> {
    let host = secure_host(manifest_url)?;
    log::info!("update check: asking {host}");
    let result = fetch_manifest(current, manifest_url);
    match &result {
        Ok(Check::Current) => log::info!("update check: {host} has nothing newer than {current}"),
        Ok(Check::Available(release)) => {
            log::info!("update check: {host} offers {}", release.version)
        }
        Err(error) => log::warn!("update check: {host} failed: {error:#}"),
    }
    result
}

fn fetch_manifest(current: &Version, manifest_url: &str) -> Result<Check> {
    let text = fetch(&manifest_agent(), manifest_url)?
        .body_mut()
        .with_config()
        .limit(MANIFEST_LIMIT)
        .read_to_string()
        .context("cannot read the update manifest")?;
    let manifest: Manifest =
        serde_json::from_str(&text).context("the update manifest is not valid")?;
    let key = platform();
    let Some(asset) = manifest.assets.get(&key) else {
        bail!("release {} has no build for {key}", manifest.version);
    };
    secure_host(&asset.url)?;
    if manifest.version <= *current {
        return Ok(Check::Current);
    }
    Ok(Check::Available(Release {
        version: manifest.version,
        url: asset.url.clone(),
        sha256: asset.sha256.to_lowercase(),
    }))
}

/// Downloads into `dir`, calling `progress(done, total)`. `Ok(None)` means cancelled.
pub fn download(
    release: &Release,
    dir: &Path,
    cancel: &AtomicBool,
    idle: Duration,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<Option<PathBuf>> {
    let host = secure_host(&release.url)?;
    let name = file_name(&release.url)?;
    let part = dir.join(format!("{name}.part"));
    let target = dir.join(name);
    log::info!("update download: {} from {host}", release.version);

    let outcome = stream(release, &part, cancel, idle, progress);
    match outcome {
        Ok(true) => {
            fs::rename(&part, &target)
                .with_context(|| format!("cannot move {} into place", part.display()))?;
            log::info!(
                "update download: {} saved to {}",
                release.version,
                target.display()
            );
            Ok(Some(target))
        }
        Ok(false) => {
            log::info!("update download: {} cancelled", release.version);
            remove_part(&part)?;
            Ok(None)
        }
        Err(error) => {
            log::warn!("update download: {} failed: {error:#}", release.version);
            if let Err(cleanup) = remove_part(&part) {
                log::warn!("update download: {cleanup:#}");
            }
            Err(error)
        }
    }
}

/// Checks a user's file against its published hash.
pub fn verify_file(path: &Path, sha256: &str) -> Result<()> {
    let mut file = File::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).with_context(|| format!("cannot read {}", path.display()))?;
    expect_hash(hasher, sha256)
}

fn stream(
    release: &Release,
    part: &Path,
    cancel: &AtomicBool,
    idle: Duration,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> Result<bool> {
    let response = fetch(&download_agent(idle), &release.url)?;
    let total = response.body().content_length();
    let (_, body) = response.into_parts();
    let (sender, receiver) = mpsc::sync_channel::<io::Result<Vec<u8>>>(4);
    thread::spawn(move || {
        let mut reader = body.into_with_config().limit(DOWNLOAD_LIMIT).reader();
        loop {
            let mut chunk = vec![0u8; 64 * 1024];
            match reader.read(&mut chunk) {
                Ok(read) => {
                    chunk.truncate(read);
                    if sender.send(Ok(chunk)).is_err() || read == 0 {
                        return;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(error));
                    return;
                }
            }
        }
    });

    let mut file =
        File::create(part).with_context(|| format!("cannot create {}", part.display()))?;
    let mut hasher = Sha256::new();
    let mut done = 0u64;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Ok(false);
        }
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(chunk)) if chunk.is_empty() => break,
            Ok(Ok(chunk)) => {
                hasher.update(&chunk);
                file.write_all(&chunk)
                    .with_context(|| format!("cannot write {}", part.display()))?;
                done += chunk.len() as u64;
                progress(done, total);
            }
            Ok(Err(error)) if is_timeout(&error) => {
                bail!("the download stalled: no data for {idle:?}")
            }
            Ok(Err(error)) => return Err(anyhow!(error).context("the download was interrupted")),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => bail!("the download thread ended early"),
        }
    }
    file.sync_all()
        .with_context(|| format!("cannot flush {}", part.display()))?;
    expect_hash(hasher, &release.sha256)?;
    Ok(true)
}

fn is_timeout(error: &io::Error) -> bool {
    let from_ureq = error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<ureq::Error>())
        .is_some_and(|inner| matches!(inner, ureq::Error::Timeout(_)));
    from_ureq
        || matches!(
            error.kind(),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
        )
}

fn remove_part(part: &Path) -> Result<()> {
    match fs::remove_file(part) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("cannot remove {}", part.display())),
    }
}

fn expect_hash(hasher: Sha256, expected: &str) -> Result<()> {
    let actual: String = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    ensure!(
        actual.eq_ignore_ascii_case(expected),
        "the file has sha256 {actual}, the release says {expected}"
    );
    Ok(())
}

fn file_name(url: &str) -> Result<&str> {
    let url = url.split_once(['?', '#']).map_or(url, |(head, _)| head);
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let path = rest.split_once('/').map_or("", |(_, path)| path);
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    ensure!(
        !name.is_empty() && name != "." && name != ".." && !name.contains(['\\', ':']),
        "{url} names no safe file"
    );
    Ok(name)
}

/// Accepts https, or http to this machine. Returns the host.
fn secure_host(url: &str) -> Result<String> {
    let uri: Uri = url.parse().with_context(|| format!("{url} is not a URL"))?;
    let authority = uri
        .authority()
        .with_context(|| format!("{url} has no host"))?;
    ensure!(
        !authority.as_str().contains('@'),
        "{url} carries credentials"
    );
    let host = authority.host();
    let local = host == "localhost"
        || host
            .trim_matches(['[', ']'])
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    ensure!(
        uri.scheme_str() == Some("https") || (uri.scheme_str() == Some("http") && local),
        "{url} is not https"
    );
    Ok(host.to_string())
}

/// GETs `url`, following redirects only to places `secure_host` accepts.
fn fetch(agent: &Agent, url: &str) -> Result<Response<Body>> {
    let mut url = url.to_string();
    for _ in 0..=MAX_REDIRECTS {
        secure_host(&url)?;
        let response = agent
            .get(&url)
            .call()
            .with_context(|| format!("cannot fetch {url}"))?;
        if !response.status().is_redirection() {
            return Ok(response);
        }
        let location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .with_context(|| format!("{url} redirects without a location"))?;
        url = if location.starts_with('/') {
            let uri: Uri = url.parse()?;
            format!(
                "{}://{}{location}",
                uri.scheme_str().unwrap_or("https"),
                uri.authority().map_or("", |a| a.as_str())
            )
        } else {
            location.to_string()
        };
    }
    bail!("too many redirects")
}

fn manifest_agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(MANIFEST_TIMEOUT))
        .max_redirects(0)
        .max_redirects_will_error(false)
        .build()
        .into()
}

/// No total timeout; a silent socket times out.
fn download_agent(idle: Duration) -> Agent {
    let config = Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_recv_response(Some(CONNECT_TIMEOUT))
        .max_redirects(0)
        .max_redirects_will_error(false)
        .build();
    Agent::with_parts(
        config,
        DefaultConnector::new().chain(IdleConnector(idle)),
        DefaultResolver::default(),
    )
}

/// Caps socket waits at `idle`; a stall frees the socket.
#[derive(Debug)]
struct IdleConnector(Duration);

impl<In: Transport> Connector<In> for IdleConnector {
    type Out = Idle<In>;

    fn connect(
        &self,
        _: &ConnectionDetails,
        chained: Option<In>,
    ) -> Result<Option<Idle<In>>, ureq::Error> {
        Ok(chained.map(|inner| Idle {
            inner,
            idle: self.0,
        }))
    }
}

#[derive(Debug)]
struct Idle<T> {
    inner: T,
    idle: Duration,
}

impl<T> Idle<T> {
    fn capped(&self, mut timeout: NextTimeout) -> NextTimeout {
        timeout.after = timeout.after.min(self.idle.into());
        timeout
    }
}

impl<T: Transport> Transport for Idle<T> {
    fn buffers(&mut self) -> &mut dyn Buffers {
        self.inner.buffers()
    }

    fn transmit_output(&mut self, amount: usize, timeout: NextTimeout) -> Result<(), ureq::Error> {
        let timeout = self.capped(timeout);
        self.inner.transmit_output(amount, timeout)
    }

    fn await_input(&mut self, timeout: NextTimeout) -> Result<bool, ureq::Error> {
        let timeout = self.capped(timeout);
        self.inner.await_input(timeout)
    }

    fn is_open(&mut self) -> bool {
        self.inner.is_open()
    }

    fn is_tls(&self) -> bool {
        self.inner.is_tls()
    }
}

#[cfg(test)]
mod tests {
    use super::{file_name, secure_host};

    #[test]
    fn plain_http_is_for_this_machine_only() {
        for ok in [
            "https://example.com/a.json",
            "http://127.0.0.1:8080/a",
            "http://localhost/a",
            "http://[::1]:9/a",
        ] {
            secure_host(ok).unwrap();
        }
        for bad in [
            "http://example.com/a",
            "http://127.0.0.1.evil.com/a",
            "http://localhost.evil.com/a",
            "http://127.0.0.1:1@192.168.1.5:80/a",
            "https://user:pw@example.com/a",
            "ftp://x/y",
            "file:///etc/passwd",
            "/relative",
        ] {
            assert!(secure_host(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn a_download_names_one_file() {
        assert_eq!(
            file_name("https://h/p/ElyDoc-0.2.0.dmg?sig=1#x").unwrap(),
            "ElyDoc-0.2.0.dmg"
        );
        for bad in [
            "https://h/p/",
            "https://h/..",
            "https://h/.",
            "https://h",
            "https://h/..\\evil.exe",
            "https://h/c:evil",
        ] {
            assert!(file_name(bad).is_err(), "{bad}");
        }
    }
}
