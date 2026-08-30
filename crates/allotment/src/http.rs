//! Shared bounded HTTP runtime for registrar transports (Gate 4F condition A).
//!
//! Policy (fail-closed):
//! - native Rust `ureq` + rustls TLS verification; NO curl / shell-out fallback
//! - host allowlist: request refused unless URL host is on the list
//! - redirects: capped; sensitive session transports use a zero-redirect agent
//! - bounded response: caller byte cap enforced while reading
//! - timeouts: global + connect
//! - explicit UA; content-type sanity check (text-ish only)
//! - sanitized errors: host visible, no URL query/body/headers embedded
//! - no logging of request or response bodies; body returned only to caller
//! - GET plus JSON POST; no shell/process surface

use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;

use ureq::ResponseExt;
use ureq::http::Uri;

/// Per-request overall timeout (covers redirects via global timeout).
pub const TIMEOUT_SECS: u64 = 15;

const MAX_REDIRECTS: u32 = 3;
const USER_AGENT: &str = "SanketIPO/0.1 (+local; allotment-discovery)";

#[derive(Debug, thiserror::Error)]
pub enum HttpPolicyError {
    /// URL rejected before any network I/O: host not on the registrar allowlist.
    #[error("host not allowed: {host}")]
    HostNotAllowed { host: String },
    /// URL could not be parsed. Sanitized: reason only, never the URL.
    #[error("invalid url")]
    InvalidUrl,
    /// Redirect ended outside the allowlist.
    #[error("redirect not allowed: {host}")]
    RedirectNotAllowed { host: String },
    #[error("too many redirects")]
    TooManyRedirects,
    /// Response body exceeded the byte cap; connection dropped.
    #[error("response body exceeded {0} bytes")]
    SizeCapExceeded(u64),
    /// HTTP status not 2xx.
    #[error("http status {0}")]
    Status(u16),
    /// Content-Type not text-ish.
    #[error("unexpected content-type: {0}")]
    UnexpectedContentType(String),
    /// Transport/TLS/DNS failure. Sanitized agent error class.
    #[error("transport: {0}")]
    Transport(String),
    /// Local policy backstop refused the call (operational, not financial).
    #[error("local http policy rate limited")]
    RateLimited,
}

impl From<ureq::Agent> for SharedHttpClient {
    fn from(agent: ureq::Agent) -> Self {
        SharedHttpClient {
            agent,
            min_interval: Duration::from_millis(500),
            last_call: std::sync::Mutex::new(None),
        }
    }
}

pub struct SharedHttpClient {
    agent: ureq::Agent,
    /// Minimal local backstop so a runaway loop cannot hammer a registrar even
    /// if a caller bypasses ProviderRateLimiter.
    /// (ponytail: single global window; per-provider policies live in
    /// ProviderRatePolicy and are wired at the service layer.)
    min_interval: Duration,
    last_call: std::sync::Mutex<Option<std::time::Instant>>,
}

impl SharedHttpClient {
    /// Process-wide shared agent (connection reuse, rustls root store).
    pub fn global() -> &'static Self {
        static CLIENT: OnceLock<SharedHttpClient> = OnceLock::new();
        CLIENT.get_or_init(|| {
            let agent = ureq::config::Config::builder()
                .timeout_global(Some(Duration::from_secs(TIMEOUT_SECS)))
                .timeout_connect(Some(Duration::from_secs(TIMEOUT_SECS)))
                .max_redirects(MAX_REDIRECTS)
                .max_redirects_will_error(true)
                .redirect_auth_headers(ureq::config::RedirectAuthHeaders::Never)
                .http_status_as_error(true)
                .user_agent(USER_AGENT)
                .build()
                .new_agent();
            SharedHttpClient {
                agent,
                // 500ms floor between calls from this process to any host.
                min_interval: Duration::from_millis(500),
                last_call: std::sync::Mutex::new(None),
            }
        })
    }

    /// Fresh cookie-isolated agent for sensitive registrar sessions. Redirects
    /// are disabled so a POST body can never cross the exact-host allowlist.
    pub fn isolated_no_redirects() -> Self {
        ureq::config::Config::builder()
            .timeout_global(Some(Duration::from_secs(TIMEOUT_SECS)))
            .timeout_connect(Some(Duration::from_secs(TIMEOUT_SECS)))
            .max_redirects(0)
            .max_redirects_will_error(true)
            .redirect_auth_headers(ureq::config::RedirectAuthHeaders::Never)
            .http_status_as_error(true)
            .user_agent(USER_AGENT)
            .build()
            .new_agent()
            .into()
    }

    /// Fail-closed GET. `allowlist` holds the exact permitted hosts for this
    /// registrar (e.g. `["ipostatus.kfintech.com"]`). Redirects are followed
    /// by the agent (capped); the final URI must remain on the allowlist.
    pub fn get(
        &self,
        url: &str,
        allowlist: &[&str],
        max_bytes: u64,
        timeout_override: u64,
    ) -> Result<String, HttpPolicyError> {
        let host = self.begin_request(url, allowlist)?;

        let response = self
            .agent
            .get(url)
            .header("Connection", "close")
            .config()
            .timeout_global(Some(Duration::from_secs(timeout_override)))
            .build()
            .call();

        let response = match response {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(code)) => return Err(HttpPolicyError::Status(code)),
            Err(ureq::Error::TooManyRedirects) => return Err(HttpPolicyError::TooManyRedirects),
            Err(ureq::Error::RedirectFailed) => {
                return Err(HttpPolicyError::RedirectNotAllowed { host });
            }
            Err(e) => return Err(HttpPolicyError::Transport(sanitize_transport(&e))),
        };

        // Redirect safety: final URI must still be on the allowlist.
        let final_host = response.get_uri().host().unwrap_or_default().to_string();
        if !allowlist.contains(&final_host.as_str()) {
            return Err(HttpPolicyError::RedirectNotAllowed { host: final_host });
        }

        read_body(response, max_bytes)
    }

    pub fn post_json(
        &self,
        url: &str,
        allowlist: &[&str],
        body: &str,
        max_bytes: u64,
        timeout_override: u64,
    ) -> Result<String, HttpPolicyError> {
        let host = self.begin_request(url, allowlist)?;
        let response = self
            .agent
            .post(url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json; charset=utf-8")
            .header("Connection", "close")
            .header("Content-Length", body.len().to_string())
            .config()
            .timeout_global(Some(Duration::from_secs(timeout_override)))
            .build()
            .send(body.as_bytes());
        let response = match response {
            Ok(response) => response,
            Err(ureq::Error::StatusCode(code)) => return Err(HttpPolicyError::Status(code)),
            Err(ureq::Error::TooManyRedirects) => return Err(HttpPolicyError::TooManyRedirects),
            Err(ureq::Error::RedirectFailed) => {
                return Err(HttpPolicyError::RedirectNotAllowed { host });
            }
            Err(error) => {
                return Err(HttpPolicyError::Transport(sanitize_transport(&error)));
            }
        };
        let final_host = response.get_uri().host().unwrap_or_default().to_string();
        if !allowlist.contains(&final_host.as_str()) {
            return Err(HttpPolicyError::RedirectNotAllowed { host: final_host });
        }
        read_body(response, max_bytes)
    }

    pub fn has_cookie(&self, domain: &str, path: &str, name: &str) -> bool {
        self.agent
            .cookie_jar_lock()
            .get(domain, path, name)
            .is_some()
    }

    fn begin_request(&self, url: &str, allowlist: &[&str]) -> Result<String, HttpPolicyError> {
        if !url.starts_with("https://") {
            return Err(HttpPolicyError::InvalidUrl);
        }
        let host = url_host(url).ok_or(HttpPolicyError::InvalidUrl)?;
        if !allowlist.contains(&host.as_str()) {
            return Err(HttpPolicyError::HostNotAllowed { host });
        }
        let mut last = self
            .last_call
            .lock()
            .map_err(|_| HttpPolicyError::Transport("policy lock poisoned".into()))?;
        if let Some(previous) = *last {
            if previous.elapsed() < self.min_interval {
                return Err(HttpPolicyError::RateLimited);
            }
        }
        *last = Some(std::time::Instant::now());
        Ok(host)
    }
}

fn read_body(
    response: ureq::http::Response<ureq::Body>,
    max_bytes: u64,
) -> Result<String, HttpPolicyError> {
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !content_type.is_empty()
        && !content_type.starts_with("text/")
        && !content_type.contains("json")
        && !content_type.contains("xml")
        && !content_type.contains("html")
    {
        return Err(HttpPolicyError::UnexpectedContentType(content_type));
    }

    let mut reader = response.into_body().into_reader();
    let mut buf: Vec<u8> = Vec::with_capacity(8 * 1024);
    let mut chunk = [0u8; 8 * 1024];
    loop {
        let n = reader
            .read(&mut chunk)
            .map_err(|e| HttpPolicyError::Transport(e.to_string()))?;
        if n == 0 {
            break;
        }
        if (buf.len() + n) as u64 > max_bytes {
            return Err(HttpPolicyError::SizeCapExceeded(max_bytes));
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8(buf).map_err(|_| HttpPolicyError::Transport("non-utf8 body".into()))
}

fn url_host(url: &str) -> Option<String> {
    Uri::try_from(url).ok()?.host().map(|h| h.to_string())
}

/// Keep only the error class; never embed URLs/headers/bodies.
fn sanitize_transport(e: &ureq::Error) -> String {
    let msg = e.to_string();
    // Split on first space+url-ish token; fall back to first 80 chars.
    let cut = msg.find("https://").or_else(|| msg.find("http://"));
    match cut {
        Some(idx) => msg[..idx].trim_end().to_string(),
        None => msg.chars().take(80).collect(),
    }
}

// ONE runnable self-check (ponytail rule): policy must fail closed.
// No network — exercises allowlist/url/scheme guards only.
#[cfg(test)]
mod tests {
    use super::*;

    fn test_client() -> SharedHttpClient {
        // Fresh instance: tests run in parallel and must not share the global
        // backstop window (which would trip RateLimited across tests).
        ureq::config::Config::builder().build().new_agent().into()
    }

    #[test]
    fn policy_fails_closed_on_disallowed_host_and_scheme() {
        let c = test_client();
        assert!(matches!(
            c.get(
                "https://evil.example.com/x",
                &["ipostatus.kfintech.com"],
                1024,
                TIMEOUT_SECS
            ),
            Err(HttpPolicyError::HostNotAllowed { .. })
        ));
        assert!(matches!(
            c.get(
                "http://ipostatus.kfintech.com/",
                &["ipostatus.kfintech.com"],
                1024,
                TIMEOUT_SECS
            ),
            Err(HttpPolicyError::InvalidUrl)
        ));
        // Suffix-trick host must not match allowlist (exact host match only).
        assert!(matches!(
            c.get(
                "https://ipostatus.kfintech.com.evil.com/",
                &["ipostatus.kfintech.com"],
                1024,
                TIMEOUT_SECS
            ),
            Err(HttpPolicyError::HostNotAllowed { .. })
        ));
    }

    #[test]
    fn local_backstop_rate_limits_rapid_calls() {
        let mut c = test_client();
        // Simulate a call that just happened: backstop must fire BEFORE any
        // network I/O (an allowlisted URL would otherwise hit the network).
        c.last_call = std::sync::Mutex::new(Some(std::time::Instant::now()));
        let res = c.get(
            "https://ipostatus.kfintech.com/",
            &["ipostatus.kfintech.com"],
            1024,
            TIMEOUT_SECS,
        );
        assert!(
            matches!(res, Err(HttpPolicyError::RateLimited)),
            "expected RateLimited when previous call is inside the window, got {res:?}"
        );
    }

    #[test]
    fn url_host_extracts_exact_host() {
        assert_eq!(
            url_host("https://ipostatus.kfintech.com/x?y=1").as_deref(),
            Some("ipostatus.kfintech.com")
        );
        // Empty string cannot parse as a URI with a host.
        assert!(url_host("").is_none());
    }

    #[test]
    fn rustls_https_transport_is_enabled() {
        // A missing ureq TLS feature panics before transport. With rustls
        // enabled this reaches the local socket and returns a normal error.
        let result = std::panic::catch_unwind(|| ureq::get("https://127.0.0.1:9").call());
        assert!(result.is_ok(), "HTTPS transport must not panic");
        assert!(result.unwrap().is_err());
    }

    #[test]
    fn sensitive_post_sessions_are_cookie_isolated_and_fail_closed() {
        let first = SharedHttpClient::isolated_no_redirects();
        let second = SharedHttpClient::isolated_no_redirects();
        let uri = Uri::from_static("https://in.mpms.mufg.com/");
        first
            .agent
            .cookie_jar_lock()
            .insert(
                ureq::Cookie::parse("session=synthetic", &uri).expect("cookie"),
                &uri,
            )
            .expect("insert");
        assert!(first.has_cookie("in.mpms.mufg.com", "/", "session"));
        assert!(!second.has_cookie("in.mpms.mufg.com", "/", "session"));
        assert!(matches!(
            first.post_json(
                "https://evil.example.com/submit",
                &["in.mpms.mufg.com"],
                "{}",
                1024,
                TIMEOUT_SECS,
            ),
            Err(HttpPolicyError::HostNotAllowed { .. })
        ));
    }
}
