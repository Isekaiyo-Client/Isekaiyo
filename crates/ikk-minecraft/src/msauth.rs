//! Legitimate Microsoft authentication for a desktop launcher (Phase 9 §5–§7).
//!
//! The flow is the OAuth 2.0 **Device Authorization Grant** against Microsoft's
//! consumer tenant, then the standard Xbox Live → XSTS → Minecraft services
//! chain. The user authenticates in THEIR browser at microsoft.com/devicelogin;
//! we never see a password, never render a fake login page, never scrape.
//!
//! Chain (every stage validated; no stage may be skipped or faked):
//! ```text
//! 1. devicecode  POST login.microsoftonline.com/consumers/oauth2/v2.0/devicecode
//! 2. token poll  POST .../oauth2/v2.0/token        (grant_type=device_code)
//! 3. XBL         POST user.auth.xboxlive.com/user/authenticate
//! 4. XSTS        POST xsts.auth.xboxlive.com/xsts/authorize
//! 5. MC login    POST api.minecraftservices.com/authentication/login_with_xbox
//! 6. profile     GET  api.minecraftservices.com/minecraft/profile
//! ```
//!
//! Secrets policy: tokens exist only inside this module's structs and are
//! returned to the caller once, for placement into the credential store.
//! `Debug` is manually implemented to redact everything. Nothing here logs.
//!
//! Configuration: the Azure application's client id comes from
//! `IKK_MS_CLIENT_ID` (set at build/deploy time by the project owner). When it
//! is absent we fail with an explicit configuration error — we never embed
//! someone else's id and never fabricate one.

use ikk_core::error::{Error, ErrorCode, Result};
use serde::{Deserialize, Serialize};

pub const DEVICECODE_URL: &str =
    "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
pub const TOKEN_URL: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
pub const XBL_URL: &str = "https://user.auth.xboxlive.com/user/authenticate";
pub const XSTS_URL: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
pub const MC_LOGIN_URL: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
pub const MC_PROFILE_URL: &str = "https://api.minecraftservices.com/minecraft/profile";

/// Scope set: sign in to Xbox Live + offline access so we hold a refresh
/// token (spec §12 silent refresh).
const SCOPE: &str = "XboxLive.signin offline_access";

fn client_id() -> Result<String> {
    std::env::var("IKK_MS_CLIENT_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            Error::new(
                ErrorCode::ConfigInvalid,
                "Microsoft sign-in is not configured on this build (missing IKK_MS_CLIENT_ID); \
                 ask the project owner for the Azure app registration",
            )
        })
}

// ---------------------------------------------------------------------------
// Wire shapes (serde only — parsing is pure and offline-testable)
// ---------------------------------------------------------------------------

/// Step-1 response: what the UI shows the user.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceCodeStart {
    pub device_code: String,
    /// Short code the user types into their browser.
    pub user_code: String,
    pub verification_uri: String,
    /// Seconds between polls (server-mandated; honor it).
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct RawDeviceCode {
    device_code: String,
    user_code: String,
    verification_uri: String,
    #[serde(default = "default_interval")]
    interval: u64,
    expires_in: u64,
}

fn default_interval() -> u64 {
    5
}

impl From<RawDeviceCode> for DeviceCodeStart {
    fn from(r: RawDeviceCode) -> Self {
        Self {
            device_code: r.device_code,
            user_code: r.user_code,
            verification_uri: r.verification_uri,
            interval: r.interval.max(1),
            expires_in: r.expires_in,
        }
    }
}

/// Parse step-1 JSON (pure; unit-testable without network).
pub fn parse_device_code(json: &str) -> Result<DeviceCodeStart> {
    let raw: RawDeviceCode = serde_json::from_str(json)
        .map_err(|e| Error::with_source(ErrorCode::AuthFailed, "malformed devicecode response", e))?;
    Ok(raw.into())
}

/// Microsoft token-endpoint error body (`{"error": "...", "error_description": "..."}`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenPollResult {
    /// `authorization_pending` | `slow_down` | `expired_token` | `access_denied` | `ok`
    pub state: String,
    /// Tokens ONLY when state == "ok". Redacted in Debug output.
    pub tokens: Option<MsTokens>,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MsTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Unix seconds when the MSA access token expires.
    pub expires_at_unix: u64,
}

impl std::fmt::Debug for MsTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MsTokens { access_token: [redacted], refresh_token: [redacted], expires_at_unix: ")
            .and_then(|f| write!(f, "{} }}", self.expires_at_unix))
    }
}

/// Pure classification of a token-endpoint response body + HTTP status.
pub fn parse_token_poll(status: u16, json: &str) -> Result<TokenPollResult> {
    #[derive(Deserialize)]
    struct ErrBody {
        error: String,
    }
    match status {
        200 => {
            #[derive(Deserialize)]
            struct OkBody {
                access_token: String,
                refresh_token: String,
                expires_in: i64,
            }
            let ok: OkBody = serde_json::from_str(json).map_err(|e| {
                Error::with_source(ErrorCode::AuthFailed, "malformed token response", e)
            })?;
            if ok.access_token.is_empty() || ok.refresh_token.is_empty() {
                return Err(Error::new(
                    ErrorCode::AuthFailed,
                    "token endpoint returned empty credentials",
                ));
            }
            Ok(TokenPollResult {
                state: "ok".into(),
                tokens: Some(MsTokens {
                    access_token: ok.access_token,
                    refresh_token: ok.refresh_token,
                    expires_at_unix: now_unix().saturating_add(ok.expires_in.clamp(0, 86_400 * 90) as u64),
                }),
            })
        }
        400 => {
            let err: ErrBody = serde_json::from_str(json).map_err(|_| {
                Error::new(ErrorCode::AuthFailed, "unrecognized token-poll failure")
            })?;
            // Only these two mean "keep waiting"; everything else is terminal.
            if matches!(err.error.as_str(), "authorization_pending" | "slow_down") {
                Ok(TokenPollResult {
                    state: err.error,
                    tokens: None,
                })
            } else if err.error == "authorization_expired" || err.error == "expired_token" {
                Ok(TokenPollResult {
                    state: "expired_token".into(),
                    tokens: None,
                })
            } else {
                Ok(TokenPollResult {
                    state: err.error, // e.g. access_denied
                    tokens: None,
                })
            }
        }
        other => Err(Error::new(
            ErrorCode::NetworkTimeout,
            format!("token endpoint returned HTTP {other}"),
        )),
    }
}

/// Xbox Live / XSTS session pieces.
#[derive(Clone, PartialEq, Eq)]
pub struct XboxSession {
    /// XSTS token (the `XBL3.0 x=<uhs>;<token>` half).
    pub xsts_token: String,
    /// User hash from DisplayClaims.
    pub uhs: String,
}

impl std::fmt::Debug for XboxSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "XboxSession {{ xsts_token: [redacted], uhs: {} }}", self.uhs)
    }
}

/// Pure parser for both XBL and XSTS responses (same shape).
pub fn parse_xbox_auth(json: &str) -> Result<XboxSession> {
    #[derive(Deserialize)]
    struct Claims {
        #[serde(default)]
        xui: Vec<std::collections::BTreeMap<String, String>>,
    }
    #[derive(Deserialize)]
    struct Body {
        #[allow(dead_code)]
        #[serde(default)]
        display_claims: Option<Claims>,
        Token: String,
    }
    let body: Body = serde_json::from_str(json)
        .map_err(|e| Error::with_source(ErrorCode::AuthFailed, "malformed Xbox auth response", e))?;
    let uhs = body
        .display_claims
        .as_ref()
        .and_then(|c| c.xui.first())
        .and_then(|m| m.get("uhs").cloned())
        .unwrap_or_default();
    if body.Token.is_empty() || uhs.is_empty() {
        return Err(Error::new(
            ErrorCode::AuthFailed,
            "Xbox auth response missing token or user hash",
        ));
    }
    Ok(XboxSession {
        xsts_token: body.Token,
        uhs,
    })
}

/// Map an XSTS rejection to an understandable cause (spec §26). Known codes:
/// 2148916233 = account has no Xbox profile; 2148916238 = child account
/// blocked; 2148916235 = region blocked.
pub fn explain_xsts_error(status: u16, json: &str) -> Error {
    let code = serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v["XErr"].as_i64());
    match code {
        Some(2148916233) => Error::new(
            ErrorCode::AuthFailed,
            "this Microsoft account has no Xbox profile; create one at xbox.com first",
        ),
        Some(2148916238) => Error::new(
            ErrorCode::AuthFailed,
            "this is a child account that cannot play Minecraft under current settings",
        ),
        Some(_) | None => Error::new(
            ErrorCode::AuthFailed,
            format!("Xbox sign-in rejected (HTTP {status})"),
        ),
    }
}

/// The final Minecraft identity produced by the chain.
#[derive(Clone, PartialEq, Eq)]
pub struct McProfile {
    pub username: String,
    /// Minecraft profile UUID (no dashes as Mojang returns it).
    pub uuid: String,
    /// Minecraft services access token for `--accessToken`.
    pub mc_access_token: String,
    pub expires_at_unix: u64,
}

impl std::fmt::Debug for McProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "McProfile {{ username: {}, uuid: {}, mc_access_token: [redacted], expires_at_unix: {} }}",
            self.username, self.uuid, self.expires_at_unix
        )
    }
}

/// Pure parsers for steps 5–6.
pub fn parse_mc_login(json: &str) -> Result<(String, u64)> {
    #[derive(Deserialize)]
    struct Body {
        access_token: String,
        expires_in: i64,
    }
    let b: Body = serde_json::from_str(json)
        .map_err(|e| Error::with_source(ErrorCode::AuthFailed, "malformed MC login response", e))?;
    if b.access_token.is_empty() {
        return Err(Error::new(
            ErrorCode::AuthFailed,
            "Minecraft services returned an empty access token — \
             check that this account owns Minecraft",
        ));
    }
    Ok((b.access_token, now_unix().saturating_add(b.expires_in.clamp(0, 86_400) as u64)))
}

pub fn parse_mc_profile(json: &str) -> Result<(String, String)> {
    #[derive(Deserialize)]
    struct Body {
        name: String,
        id: String,
    }
    let b: Body = serde_json::from_str(json).map_err(|e| {
        Error::with_source(
            ErrorCode::AuthFailed,
            "profile unavailable — does this account own Minecraft Java Edition?",
            e,
        )
    })?;
    if b.name.is_empty() || b.id.is_empty() {
        return Err(Error::new(
            ErrorCode::AuthFailed,
            "Minecraft profile is missing its name or id — entitlement check failed",
        ));
    }
    Ok((b.name, b.id))
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Network steps — thin, each takes &ureq::Agent, never logs
// ---------------------------------------------------------------------------

fn post_form(agent: &ureq::Agent, url: &str, form: &str) -> Result<(u16, String)> {
    let response = agent
        .post(url)
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(form);
    read_response(response, url)
}

fn post_json(agent: &ureq::Agent, url: &str, body: &str) -> Result<(u16, String)> {
    let response = agent
        .post(url)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json")
        .send_string(body);
    read_response(response, url)
}

/// Normalize any HTTP outcome into `(status, body)`; non-2xx statuses are
/// returned (not errored) because auth flows classify them themselves.
fn read_response(
    response: std::result::Result<ureq::Response, ureq::Error>,
    url: &str,
) -> Result<(u16, String)> {
    let (status, response) = match response {
        Ok(response) => (response.status(), response),
        Err(ureq::Error::Status(status, response)) => (status, response),
        Err(other) => {
            return Err(Error::with_source(
                ErrorCode::NetworkTimeout,
                format!("network error contacting {url}"),
                other,
            ))
        }
    };
    let text = response.into_string().map_err(|e| {
        Error::with_source(
            ErrorCode::AuthFailed,
            format!("unreadable response body from {url}"),
            e,
        )
    })?;
    Ok((status, text))
}

fn form_encode(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencode(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Step 1: request a device code.
pub fn start_device_flow(agent: &ureq::Agent) -> Result<DeviceCodeStart> {
    let cid = client_id()?;
    let (status, body) = post_form(
        agent,
        DEVICECODE_URL,
        &form_encode(&[("client_id", &cid), ("scope", SCOPE)]),
    )?;
    if status != 200 {
        return Err(Error::new(
            ErrorCode::ConfigInvalid,
            format!("device-code endpoint rejected our client configuration (HTTP {status})"),
        ));
    }
    parse_device_code(&body)
}

/// One poll of step 2. Returns pending/slow-down/expired/denied states; the
/// caller drives the retry loop (bounded, honoring `interval`).
pub fn poll_device_flow(agent: &ureq::Agent, device_code: &str) -> Result<TokenPollResult> {
    let cid = client_id()?;
    let (status, body) = post_form(
        agent,
        TOKEN_URL,
        &form_encode(&[
            ("client_id", &cid),
            (
                "grant_type",
                "urn:ietf:params:oauth:grant-type:device_code",
            ),
            ("device_code", device_code),
        ]),
    )?;
    parse_token_poll(status, &body)
}

/// Silent refresh (spec §12): exchange the stored refresh token for fresh MSA
/// tokens. Bounded — callers attempt ONCE per launch preparation.
pub fn refresh_tokens(agent: &ureq::Agent, refresh_token: &str) -> Result<MsTokens> {
    let cid = client_id()?;
    let (status, body) = post_form(
        agent,
        TOKEN_URL,
        &form_encode(&[
            ("client_id", &cid),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ]),
    )?;
    match parse_token_poll(status, &body)? {
        TokenPollResult {
            state,
            tokens: Some(tokens),
        } if state == "ok" => Ok(tokens),
        _ => Err(Error::new(
            ErrorCode::AuthTokenExpired,
            "silent refresh was rejected — reauthentication required",
        )),
    }
}

/// Steps 3–6: XBL → XSTS → Minecraft login → profile.
pub fn complete_minecraft_chain(
    agent: &ureq::Agent,
    msa: &MsTokens,
) -> Result<McProfile> {
    // 3. Xbox Live user token.
    let xbl_body = format!(
        "{{\"Properties\":{{\"AuthMethod\":\"RPS\",\"SiteName\":\"user.auth.xboxlive.com\",\"RpsTicket\":\"d={}\"}},\"RelyingParty\":\"http://auth.xboxlive.com\",\"TokenType\":\"JWT\"}}",
        msa.access_token
    );
    let (status, body) = post_json(agent, XBL_URL, &xbl_body)?;
    if status != 200 {
        return Err(explain_xsts_error(status, &body));
    }
    let xbl = parse_xbox_auth(&body)?;

    // 4. XSTS authorization.
    let xsts_body = format!(
        "{{\"Properties\":{{\"SandboxId\":\"RETAIL\",\"UserTokens\":[\"{}\"]}},\"RelyingParty\":\"rp://api.minecraftservices.com/\",\"TokenType\":\"JWT\"}}",
        xbl.xsts_token
    );
    let (status, body) = post_json(agent, XSTS_URL, &xsts_body)?;
    if status != 200 {
        return Err(explain_xsts_error(status, &body));
    }
    let xsts = parse_xbox_auth(&body)?;

    // 5. Minecraft services login.
    let login_body = format!(
        "{{\"identityToken\":\"XBL3.0 x={};{}\"}}",
        xsts.uhs, xsts.xsts_token
    );
    let (status, body) = post_json(agent, MC_LOGIN_URL, &login_body)?;
    if status != 200 {
        return Err(Error::new(
            ErrorCode::AuthFailed,
            format!("Minecraft sign-in rejected (HTTP {status})"),
        ));
    }
    let (mc_token, expires_at) = parse_mc_login(&body)?;

    // 6. Profile (entitlement proof).
    let profile_text = get_bearer(agent, MC_PROFILE_URL, &mc_token)?;
    let (username, uuid) = parse_mc_profile(&profile_text)?;

    Ok(McProfile {
        username,
        uuid,
        mc_access_token: mc_token,
        expires_at_unix: expires_at,
    })
}

fn get_bearer(agent: &ureq::Agent, url: &str, token: &str) -> Result<String> {
    match agent.get(url).set("Authorization", &format!("Bearer {token}")).call() {
        Ok(response) => response.into_string().map_err(|e| {
            Error::with_source(ErrorCode::AuthFailed, "unreadable profile body", e)
        }),
        Err(ureq::Error::Status(404, _)) => Err(Error::new(
            ErrorCode::AuthFailed,
            "no Minecraft profile for this account (Java Edition not owned?)",
        )),
        Err(e) => Err(Error::with_source(
            ErrorCode::NetworkTimeout,
            format!("network error fetching profile"),
            e,
        )),
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn device_code_parses_with_defaults() {
        let json = r#"{
            "device_code": "DEV123",
            "user_code": "ABCD-EFGH",
            "verification_uri": "https://microsoft.com/devicelogin",
            "expires_in": 900
        }"#;
        let start = parse_device_code(json).unwrap();
        assert_eq!(start.device_code, "DEV123");
        assert_eq!(start.interval, 5, "server default applied");
        assert_eq!(start.user_code, "ABCD-EFGH");
    }

    #[test]
    fn token_poll_classifies_all_states() {
        let pending = parse_token_poll(
            400,
            r#"{"error":"authorization_pending","error_description":"wait"}"#,
        )
        .unwrap();
        assert_eq!(pending.state, "authorization_pending");
        assert!(pending.tokens.is_none());

        let denied = parse_token_poll(
            400,
            r#"{"error":"access_denied","error_description":"no"}"#,
        )
        .unwrap();
        assert_eq!(denied.state, "access_denied");

        let ok = parse_token_poll(
            200,
            r#"{"access_token":"at","refresh_token":"rt","expires_in":3600}"#,
        )
        .unwrap();
        assert_eq!(ok.state, "ok");
        let t = ok.tokens.unwrap();
        assert!(t.expires_at_unix > now_unix());

        // Malformed success bodies are errors, never silently accepted.
        assert!(parse_token_poll(200, r#"{"access_token":""}"#).is_err());
    }

    #[test]
    fn debug_never_leaks_secrets() {
        let tokens = MsTokens {
            access_token: "SUPER-SECRET-AT".into(),
            refresh_token: "SUPER-SECRET-RT".into(),
            expires_at_unix: 1,
        };
        let rendered = format!("{tokens:?}");
        assert!(!rendered.contains("SUPER-SECRET"));
        assert!(rendered.contains("[redacted]"));

        let session = XboxSession {
            xsts_token: "XSTS-SECRET".into(),
            uhs: "uhs-value".into(),
        };
        assert!(!format!("{session:?}").contains("XSTS-SECRET"));

        let profile = McProfile {
            username: "Steve".into(),
            uuid: "uuid".into(),
            mc_access_token: "MC-SECRET".into(),
            expires_at_unix: 1,
        };
        assert!(!format!("{profile:?}").contains("MC-SECRET"));
    }

    #[test]
    fn xbox_and_minecraft_parsers_validate_shapes() {
        let session = parse_xbox_auth(
            r#"{"DisplayClaims":{"xui":[{"uhs":"UHS1"}]},"Token":"TOK1"}"#,
        )
        .unwrap();
        assert_eq!(session.uhs, "UHS1");
        assert_eq!(session.xsts_token, "TOK1");

        // Missing uhs is rejected rather than defaulted.
        assert!(parse_xbox_auth(r#"{"DisplayClaims":{"xui":[{}]},"Token":"T"}"#).is_err());

        let (tok, exp) = parse_mc_login(r#"{"access_token":"mc","expires_in":100}"#).unwrap();
        assert_eq!(tok, "mc");
        assert!(exp > now_unix());

        let (name, id) =
            parse_mc_profile(r#"{"name":"Steve","id":"abc123def4567890abcdef1234567890"}"#)
                .unwrap();
        assert_eq!((name.as_str(), id.as_str()), ("Steve", "abc123def4567890abcdef1234567890"));
    }

    #[test]
    fn xsts_errors_map_to_understandable_causes() {
        let no_xbox = explain_xsts_error(401, r#"{"XErr":2148916233}"#);
        assert!(no_xbox.to_string().contains("no Xbox profile"));
        let child = explain_xsts_error(401, r#"{"XErr":2148916238}"#);
        assert!(child.to_string().contains("child"));
        let generic = explain_xsts_error(500, r#"{"XErr":999}"#);
        assert!(generic.to_string().contains("HTTP 500"));
    }

    #[test]
    fn urlencoding_is_correct_for_oauth_forms() {
        assert_eq!(urlencode("plain"), "plain");
        assert_eq!(urlencode("a b&c=d"), "a%20b%26c%3Dd");
        assert_eq!(form_encode(&[("k", "v/w"), ("grant", "x y")]), "k=v%2Fw&grant=x%20y");
    }
}
