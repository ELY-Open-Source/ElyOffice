//! The real logger at trace, around token requests.
mod oauth_fake;

use ely_mail_engine::oauth::refresh;
use ely_platform::AppId;
use oauth_fake::{endpoint, provider, signed_in};

const CODE_TOKEN: &str = "PRIVATE_FIRST";
const ROTATED: &str = "PRIVATE_ROTATED";

#[tokio::test]
async fn trace_logs_keep_no_token() {
    let dir = tempfile::tempdir().unwrap();
    // The only test here; no thread reads env.
    unsafe { std::env::set_var("ELY_LOG", "trace") };
    ely_platform::logging::init_in(AppId::Mail, dir.path()).unwrap();

    let body = format!(
        r#"{{"access_token":"{CODE_TOKEN}","expires_in":60,"refresh_token":"{CODE_TOKEN}"}}"#
    );
    signed_in(body).await.unwrap();
    let body =
        format!(r#"{{"access_token":"{ROTATED}","expires_in":60,"refresh_token":"{ROTATED}"}}"#);
    let (token, _) = endpoint(200, body);
    refresh(&provider(token), CODE_TOKEN)
        .await
        .unwrap();

    log::logger().flush();
    let logged = std::fs::read_to_string(dir.path().join("ely.log")).unwrap();
    assert!(logged.contains("oauth: test token granted"), "{logged}");
    for secret in [CODE_TOKEN, ROTATED, "PRIVATE"] {
        assert!(
            !logged.contains(secret),
            "{secret} reached the log:
{logged}"
        );
    }
    assert!(
        !logged.contains(" ureq"),
        "HTTP library logged:
{logged}"
    );
}
