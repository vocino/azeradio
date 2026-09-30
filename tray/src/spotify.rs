use crate::config::{self, SpotifyAuth};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Makes sure auth.access_token is fresh, refreshing or running the
/// first-time browser flow as needed.
pub fn ensure_token(auth: &mut SpotifyAuth) -> Result<(), String> {
    if !auth.access_token.is_empty() && now_unix() < auth.expires_at.saturating_sub(120) {
        return Ok(());
    }
    if auth.refresh_token.is_empty() {
        return oauth_flow(auth);
    }

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://accounts.spotify.com/api/token")
        .basic_auth(&auth.client_id, Some(&auth.client_secret))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", auth.refresh_token.as_str()),
        ])
        .send()
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("spotify token refresh failed: {}", res.status()));
    }
    let v: serde_json::Value = res.json().map_err(|e| e.to_string())?;
    auth.access_token = v["access_token"].as_str().unwrap_or("").to_string();
    if let Some(rt) = v["refresh_token"].as_str() {
        auth.refresh_token = rt.to_string();
    }
    auth.expires_at = now_unix() + v["expires_in"].as_u64().unwrap_or(3600);
    config::save_auth(auth);
    Ok(())
}

/// First-run login: opens the browser at Spotify, catches the redirect on
/// localhost, exchanges the code for tokens. Needs client_id/client_secret
/// already in spotify.json (see docs/SETUP.md).
pub fn oauth_flow(auth: &mut SpotifyAuth) -> Result<(), String> {
    if auth.client_id.is_empty() || auth.client_secret.is_empty() {
        return Err(
            "put your Spotify client_id and client_secret in spotify.json first (docs/SETUP.md)"
                .into(),
        );
    }
    let server = tiny_http::Server::http("127.0.0.1:8899").map_err(|e| e.to_string())?;
    let redirect = "http://127.0.0.1:8899/callback";
    let scopes = "user-modify-playback-state%20user-read-playback-state";
    let auth_url = format!(
        "https://accounts.spotify.com/authorize?response_type=code&client_id={}&scope={}&redirect_uri={}",
        auth.client_id, scopes, redirect
    );
    open::that(&auth_url).map_err(|e| e.to_string())?;

    let mut code: Option<String> = None;
    for request in server.incoming_requests() {
        let url = request.url().to_string();
        if let Some(q) = url.split('?').nth(1) {
            for pair in q.split('&') {
                let mut kv = pair.splitn(2, '=');
                if kv.next() == Some("code") {
                    code = kv.next().map(|s| s.to_string());
                }
            }
        }
        let _ = request.respond(tiny_http::Response::from_string(
            "Azeradio is connected. You can close this tab.",
        ));
        if code.is_some() {
            break;
        }
    }
    let code = code.ok_or("spotify did not return a code")?;

    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://accounts.spotify.com/api/token")
        .basic_auth(&auth.client_id, Some(&auth.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect),
        ])
        .send()
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("spotify token exchange failed: {}", res.status()));
    }
    let v: serde_json::Value = res.json().map_err(|e| e.to_string())?;
    auth.access_token = v["access_token"].as_str().unwrap_or("").to_string();
    auth.refresh_token = v["refresh_token"].as_str().unwrap_or("").to_string();
    auth.expires_at = now_unix() + v["expires_in"].as_u64().unwrap_or(3600);
    config::save_auth(auth);
    Ok(())
}

/// Starts playing a playlist/album URI. Needs Spotify Premium and an active
/// device (open Spotify somewhere first).
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
        403 => Err("spotify refused: playback control needs Premium".into()),
        404 => Err("no active Spotify device; open Spotify on something first".into()),
        s => Err(format!("spotify play failed with status {s}")),
    }
}
