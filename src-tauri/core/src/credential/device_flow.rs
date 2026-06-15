//! GitHub OAuth device flow. Fully implemented; activates once `CLIENT_ID` is set
//! to a registered GitHub OAuth App (Device Flow enabled, no client secret needed).
//! Until then `start()` returns a clear "not configured" error and the gh/PAT paths
//! cover authentication.

use crate::error::{AppError, Result};
use crate::sensitive::Sensitive;
use serde::Deserialize;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Set this to the OAuth App client id (public; ship in the binary). Empty = unconfigured.
pub const CLIENT_ID: &str = "Ov23ligjZkyhamxAek0U";
/// Scope required to read and clone private repos.
const SCOPE: &str = "repo";

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default = "default_interval")]
    pub interval: u64,
    pub expires_in: u64,
}

fn default_interval() -> u64 {
    5
}

pub fn is_configured() -> bool {
    !CLIENT_ID.is_empty()
}

/// Request a device + user code to show the user.
pub fn start() -> Result<DeviceCode> {
    if !is_configured() {
        return Err(AppError::Auth(
            "device flow not configured: register a GitHub OAuth App and set credential::device_flow::CLIENT_ID".into(),
        ));
    }
    let resp = ureq::post("https://github.com/login/device/code")
        .set("Accept", "application/json")
        .send_form(&[("client_id", CLIENT_ID), ("scope", SCOPE)])
        .map_err(|e| AppError::Network(e.to_string()))?;
    resp.into_json::<DeviceCode>()
        .map_err(|e| AppError::Network(format!("device/code parse: {e}")))
}

#[derive(Debug, Deserialize)]
struct TokenResp {
    access_token: Option<String>,
    error: Option<String>,
    #[serde(default = "default_interval")]
    interval: u64,
}

/// Poll for the access token until the user authorizes, it expires, or `cancel`
/// returns true. Blocking - call from a background thread.
pub fn poll_blocking(code: &DeviceCode, mut cancel: impl FnMut() -> bool) -> Result<Sensitive<String>> {
    let deadline = Instant::now() + Duration::from_secs(code.expires_in);
    let mut interval = code.interval.max(1);
    while Instant::now() < deadline {
        if cancel() {
            return Err(AppError::Auth("sign-in cancelled".into()));
        }
        sleep(Duration::from_secs(interval));
        let resp = ureq::post("https://github.com/login/oauth/access_token")
            .set("Accept", "application/json")
            .send_form(&[
                ("client_id", CLIENT_ID),
                ("device_code", &code.device_code),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .map_err(|e| AppError::Network(e.to_string()))?;
        let body: TokenResp = resp
            .into_json()
            .map_err(|e| AppError::Network(format!("token parse: {e}")))?;
        if let Some(token) = body.access_token {
            return Ok(Sensitive(token));
        }
        match body.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval = body.interval.max(interval + 5),
            Some("expired_token") => return Err(AppError::Auth("sign-in code expired".into())),
            Some("access_denied") => return Err(AppError::Auth("sign-in denied".into())),
            Some(other) => return Err(AppError::Auth(format!("device flow: {other}"))),
            None => {}
        }
    }
    Err(AppError::Auth("sign-in timed out".into()))
}
