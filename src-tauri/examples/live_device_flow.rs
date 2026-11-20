//! Live check of the GitHub device flow.
//!
//! Every other test in this repository runs against a mock. This one talks to
//! GitHub, which is the only way to find out whether the app registration
//! actually has device flow enabled and whether the response shapes match what
//! the client expects.
//!
//! It is an example rather than a test so it never runs in CI — it needs a
//! client id and, to complete, a human.
//!
//! ```text
//! PULSE_GITHUB_CLIENT_ID=Iv23li… cargo run --example live_device_flow
//! PULSE_GITHUB_CLIENT_ID=Iv23li… cargo run --example live_device_flow -- --wait
//! ```
//!
//! `--wait` polls until the code is authorised (or expires) and then confirms
//! the token with `GET /user`. It prints the login and the token's expiry, and
//! never the token itself.

use std::time::Duration;

use pulse_lib::auth::oauth::{OAuthClient, PollOutcome};
use pulse_lib::github::{build_client, Endpoints};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = std::env::var("PULSE_GITHUB_CLIENT_ID")
        .map_err(|_| "set PULSE_GITHUB_CLIENT_ID to the app's client id")?;
    let wait = std::env::args().any(|argument| argument == "--wait");

    let http = build_client()?;
    let client = OAuthClient::new(http.clone(), Endpoints::github(), Some(client_id.clone()));

    if std::env::args().any(|argument| argument == "--errors") {
        return check_error_paths(http, &client_id).await;
    }

    println!("requesting a device code from github.com…");
    let grant = client.request_device_code(&[]).await?;

    println!();
    println!("  verification URI : {}", grant.verification_uri);
    println!("  user code        : {}", grant.user_code);
    println!("  polling interval : {:?}", grant.interval);
    println!("  expires at       : {}", grant.expires_at);
    println!();

    // One poll proves the polling shape without needing a human.
    match client.poll_device_code(&grant.device_code).await? {
        PollOutcome::Pending => println!("first poll: authorization_pending (as expected)"),
        PollOutcome::SlowDown => println!("first poll: slow_down (github wants a longer interval)"),
        PollOutcome::Granted(_) => println!("first poll: already granted"),
    }

    if !wait {
        println!();
        println!("device code request and polling shape verified.");
        println!("re-run with --wait to complete the flow by hand.");
        return Ok(());
    }

    println!();
    println!(
        "open {} and enter {}",
        grant.verification_uri, grant.user_code
    );
    println!("waiting up to 15 minutes…");

    let mut interval = grant.interval;

    loop {
        if chrono::Utc::now() >= grant.expires_at {
            return Err("the device code expired before it was authorised".into());
        }

        tokio::time::sleep(interval).await;

        match client.poll_device_code(&grant.device_code).await {
            Ok(PollOutcome::Pending) => {}
            Ok(PollOutcome::SlowDown) => {
                interval += Duration::from_secs(5);
                println!("slow_down: interval is now {interval:?}");
            }
            Ok(PollOutcome::Granted(tokens)) => {
                println!();
                println!("AUTHORISED");
                println!(
                    "  access token  : present, {} chars",
                    tokens.access_token.len()
                );
                println!("  expires at    : {:?}", tokens.expires_at);
                println!(
                    "  refresh token : {}",
                    if tokens.can_refresh() {
                        "present"
                    } else {
                        "absent"
                    }
                );

                // Confirm the token works without printing it.
                match client.fetch_viewer(&tokens.access_token).await {
                    Ok(viewer) => println!("  authenticated : {}", viewer.login),
                    Err(error) => println!("  GET /user failed: {error}"),
                }

                return Ok(());
            }
            Err(error) => return Err(format!("the flow failed: {error}").into()),
        }
    }
}
/// Error paths, checked against GitHub rather than a mock.
///
/// What a mock cannot tell you is whether GitHub answers with the `error` field
/// the client matches on, or something else entirely.
async fn check_error_paths(
    http: reqwest::Client,
    client_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("checking error paths against github.com...");

    let bogus = OAuthClient::new(
        http.clone(),
        Endpoints::github(),
        Some("Iv23li000000000000000000".to_owned()),
    );

    match bogus.request_device_code(&[]).await {
        Ok(_) => println!("  bogus client id   : UNEXPECTEDLY accepted"),
        Err(error) => println!("  bogus client id   : {} ({})", error.code(), error),
    }

    let real = OAuthClient::new(http, Endpoints::github(), Some(client_id.to_owned()));

    match real.poll_device_code("pulse-not-a-real-device-code").await {
        Ok(outcome) => println!("  bogus device code : UNEXPECTEDLY accepted ({outcome:?})"),
        Err(error) => println!("  bogus device code : {} ({})", error.code(), error),
    }

    Ok(())
}
