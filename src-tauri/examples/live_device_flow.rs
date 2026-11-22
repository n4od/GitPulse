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
//! GITPULSE_GITHUB_CLIENT_ID=Iv23li… cargo run --example live_device_flow
//! GITPULSE_GITHUB_CLIENT_ID=Iv23li… cargo run --example live_device_flow -- --wait
//! ```
//!
//! `--wait` polls until the code is authorised (or expires) and then confirms
//! the token with `GET /user`. It prints the login and the token's expiry, and
//! never the token itself.

use std::time::Duration;

use gitpulse_lib::auth::oauth::{OAuthClient, PollOutcome};
use gitpulse_lib::github::{build_client, Endpoints};

struct KeychainToken;

#[async_trait::async_trait]
impl gitpulse_lib::github::TokenProvider for KeychainToken {
    async fn access_token(&self) -> Result<secrecy::SecretString, gitpulse_lib::error::AppError> {
        let token = gitpulse_lib::secrets::get("github-access-token")?
            .ok_or_else(|| gitpulse_lib::error::AppError::AuthRequired)?;

        Ok(secrecy::SecretString::new(token.into_boxed_str()))
    }
}

/// Exercises the exact endpoint layer the app uses, with the real token.
async fn check_api_with_token() -> Result<(), Box<dyn std::error::Error>> {
    let budget = std::sync::Arc::new(gitpulse_lib::github::RateBudget::new(3600.0, 30.0));
    let client = gitpulse_lib::github::GithubClient::new(
        build_client()?,
        Endpoints::github(),
        std::sync::Arc::new(KeychainToken),
        budget,
    );

    let repos = client.repos(1, 100).await?;
    println!("GET /user/repos: {} repositories", repos.len());
    for repo in repos.iter().take(5) {
        println!("  {}", repo.full_name);
    }

    // Any repository the token can read; the default is public, so the check runs
    // without anyone's private repository being named in the source.
    let sample =
        std::env::var("GITPULSE_CHECK_REPO").unwrap_or_else(|_| "octocat/Hello-World".to_owned());

    let runs = client.runs(&sample, 20).await?;
    println!("GET /actions/runs for {sample}: {} runs", runs.len());
    for run in runs.iter().take(5) {
        println!(
            "  #{} {} {} {:?}",
            run.run_number.unwrap_or(0),
            run.name.as_deref().unwrap_or("?"),
            run.status,
            run.conclusion
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = std::env::var("GITPULSE_GITHUB_CLIENT_ID")
        .map_err(|_| "set GITPULSE_GITHUB_CLIENT_ID to the app's client id")?;
    let wait = std::env::args().any(|argument| argument == "--wait");

    let http = build_client()?;
    let client = OAuthClient::new(http.clone(), Endpoints::github(), Some(client_id.clone()));

    if std::env::args().any(|argument| argument == "--errors") {
        return check_error_paths(http, &client_id).await;
    }

    if std::env::args().any(|argument| argument == "--api-me") {
        return check_api_with_token().await;
    }

    if std::env::args().any(|argument| argument == "--me") {
        let token = gitpulse_lib::secrets::get("github-access-token")?
            .ok_or("no access token in the keychain; sign in first")?;
        let viewer = client.fetch_viewer(&token).await?;

        println!("GET /user with the stored keychain token: {}", viewer.login);
        return Ok(());
    }

    if std::env::args().any(|argument| argument == "--api") {
        let response = http
            .get("https://api.github.com/rate_limit")
            .header(reqwest::header::USER_AGENT, "gitpulse-live-check")
            .send()
            .await?;

        println!("api.github.com via reqwest: {}", response.status());
        println!("body bytes: {}", response.bytes().await?.len());
        return Ok(());
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

    match real
        .poll_device_code("gitpulse-not-a-real-device-code")
        .await
    {
        Ok(outcome) => println!("  bogus device code : UNEXPECTEDLY accepted ({outcome:?})"),
        Err(error) => println!("  bogus device code : {} ({})", error.code(), error),
    }

    Ok(())
}
