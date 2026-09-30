use crate::config::{self, SpotifyAuth};
use base64::Engine as _;
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// Bring your own key: every user pastes their own client ID into
/// spotify.json (docs/SETUP.md). No client secret anywhere — desktop apps
/// use Authorization Code + PKCE, which is Spotify's flow for apps that
/// cannot keep a secret.
const SCOPES: &str = "user-modify-playback-state user-read-playback-state";
const REDIRECT: &str = "http://127.0.0.1:8899/callback";

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Random 64-char verifier from the PKCE unreserved alphabet.
fn gen_verifier() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 48];
    rand::rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn challenge(verifier: &str) -> String {
    let hash = Sha256::digest(verifier.as_bytes());
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash)
}

fn store_tokens(auth: &mut SpotifyAuth, v: &serde_json::Value) {
    auth.access_token = v["access_token"].as_str().unwrap_or("").to_string();
    if let Some(rt) = v["refresh_token"].as_str() {
        if !rt.is_empty() {
            auth.refresh_token = rt.to_string();
        }
    }
    auth.expires_at = now_unix() + v["expires_in"].as_u64().unwrap_or(3600);
    config::save_auth(auth);
}

/// Makes sure auth.access_token is fresh: refreshes it, or runs the browser
/// login when there is nothing to refresh (first run, or the refresh token
/// expired — Spotify expires those now, so re-login is a normal event).
pub fn ensure_token(auth: &mut SpotifyAuth) -> Result<(), String> {
    if auth.client_id.is_empty() {
        return Err("put your Spotify client_id in spotify.json first (docs/SETUP.md)".into());
    }
    if !auth.access_token.is_empty() && now_unix() < auth.expires_at.saturating_sub(120) {
        return Ok(());
    }
    if auth.refresh_token.is_empty() {
        return oauth_flow(auth);
    }

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", auth.refresh_token.as_str()),
            ("client_id", auth.client_id.as_str()),
        ])
        .send()
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        eprintln!("[azeradio] Spotify refresh failed, opening browser to log in again");
        return oauth_flow(auth);
    }
    let v: serde_json::Value = res.json().map_err(|e| e.to_string())?;
    store_tokens(auth, &v);
    Ok(())
}

/// Browser login: opens Spotify, catches the redirect on localhost, trades
/// the code plus the PKCE verifier for tokens. No client secret involved.
pub fn oauth_flow(auth: &mut SpotifyAuth) -> Result<(), String> {
    if auth.client_id.is_empty() {
        return Err("put your Spotify client_id in spotify.json first (docs/SETUP.md)".into());
    }
    let verifier = gen_verifier();
    let auth_url = format!(
        "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}&code_challenge_method=S256&code_challenge={}",
        auth.client_id,
        SCOPES.replace(' ', "%20"),
        REDIRECT,
        challenge(&verifier)
    );
    let server = tiny_http::Server::http("127.0.0.1:8899").map_err(|e| e.to_string())?;
    open::that(&auth_url).map_err(|e| e.to_string())?;

    let mut code: Option<String> = None;
    let mut denied = false;
    for request in server.incoming_requests() {
        let url = request.url().to_string();
        if let Some(q) = url.split('?').nth(1) {
            for pair in q.split('&') {
                let mut kv = pair.splitn(2, '=');
                match kv.next() {
                    Some("code") => code = kv.next().map(|s| s.to_string()),
                    Some("error") => denied = true,
                    _ => {}
                }
            }
        }
        let _ = request.respond(tiny_http::Response::from_string(
            "Azeradio is connected. You can close this tab.",
        ));
        if code.is_some() || denied {
            break;
        }
    }
    if denied {
        return Err("Spotify login was denied".into());
    }
    let code = code.ok_or("Spotify did not return a code")?;

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://accounts.spotify.com/api/token")
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", REDIRECT),
            ("client_id", auth.client_id.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send()
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Spotify token exchange failed: {}", res.status()));
    }
    let v: serde_json::Value = res.json().map_err(|e| e.to_string())?;
    store_tokens(auth, &v);
    Ok(())
}

/// Starts playing a playlist/album/artist URI. Needs Spotify Premium and an
/// active device (open Spotify somewhere first).
pub fn play_context(auth: &SpotifyAuth, uri: &str, device_id: Option<&str>) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let mut url = "https://api.spotify.com/v1/me/player/play".to_string();
    if let Some(d) = device_id {
        url.push_str(&format!("?device_id={d}"));
    }
    let res = client
        .put(&url)
        .bearer_auth(&auth.access_token)
        .json(&serde_json::json!({ "context_uri": uri }))
        .send()
        .map_err(|e| e.to_string())?;
    match res.status().as_u16() {
        200 | 204 => Ok(()),
        403 => Err("Spotify refused: playback control needs Premium".into()),
        404 => Err("no active Spotify device; open Spotify on something first".into()),
        s => Err(format!("Spotify play failed with status {s}")),
    }
}
