//! GraphQL: the lifetime contributions graph (including private activity) and the
//! viewer's identity, for the radial impact tree. One batched request fetches the
//! latest-year calendar plus a per-year total for every year since GitHub launched
//! (years before the account existed simply return zero), so the whole lifetime
//! number arrives in a single round trip. The avatar is fetched separately and
//! inlined as a `data:` URI so the webview never has to reach the network.

use super::GithubClient;
use crate::error::{AppError, Result};
use crate::types::{ContributionDay, ProfileStats};
use serde_json::{json, Value};
use std::io::Read;

const GQL: &str = "https://api.github.com/graphql";
const UA: &str = "master-diff (https://github.com/Jam-Sw/master-diff)";
/// GitHub launched in 2008; no contributions predate it.
const FIRST_YEAR: i32 = 2008;

impl GithubClient {
    fn post_graphql(&self, query: &str) -> Result<Value> {
        let resp = ureq::post(GQL)
            .set("Authorization", &format!("Bearer {}", self.token.expose()))
            .set("User-Agent", UA)
            .set("Content-Type", "application/json")
            .send_json(json!({ "query": query }));
        match resp {
            Ok(r) => r
                .into_json()
                .map_err(|e| AppError::Network(format!("graphql parse: {e}"))),
            Err(ureq::Error::Status(401, _)) => {
                Err(AppError::Auth("GitHub rejected the token (401)".into()))
            }
            Err(ureq::Error::Status(403, _)) => {
                Err(AppError::RateLimited("GitHub GraphQL rate limit or scope".into()))
            }
            Err(ureq::Error::Status(code, r)) => {
                Err(AppError::Network(format!("GitHub graphql {code}: {}", r.status_text())))
            }
            Err(ureq::Error::Transport(t)) => Err(AppError::Network(t.to_string())),
        }
    }

    /// The viewer's identity + contributions graph for the impact tree.
    pub fn fetch_profile(&self) -> Result<ProfileStats> {
        let now_year = current_year();
        let mut year_aliases = String::new();
        for y in FIRST_YEAR..=now_year {
            year_aliases.push_str(&format!(
                "y{y}: contributionsCollection(from: \"{y}-01-01T00:00:00Z\", to: \"{y}-12-31T23:59:59Z\") {{ contributionCalendar {{ totalContributions }} }}\n"
            ));
        }
        let query = format!(
            "query {{\n  viewer {{\n    login\n    name\n    avatarUrl(size: 240)\n    createdAt\n    contributionsCollection {{\n      contributionCalendar {{\n        totalContributions\n        weeks {{ contributionDays {{ date contributionCount color }} }}\n      }}\n    }}\n    {year_aliases}\n  }}\n}}"
        );

        let v = self.post_graphql(&query)?;
        let viewer = v
            .pointer("/data/viewer")
            .filter(|x| !x.is_null())
            .ok_or_else(|| {
                let errs = v.get("errors").map(|e| e.to_string()).unwrap_or_default();
                AppError::Network(format!("graphql: no viewer data {errs}"))
            })?;

        let login = str_at(viewer, "login").unwrap_or_default();
        let name = str_at(viewer, "name");
        let avatar_url = str_at(viewer, "avatarUrl");
        let created_at = str_at(viewer, "createdAt").unwrap_or_default();
        let created_year: i32 = created_at
            .get(0..4)
            .and_then(|s| s.parse().ok())
            .unwrap_or(FIRST_YEAR);

        let cal = viewer.pointer("/contributionsCollection/contributionCalendar");
        let last_year_contributions = cal
            .and_then(|c| c.get("totalContributions"))
            .and_then(Value::as_u64)
            .unwrap_or(0);

        let mut calendar = Vec::new();
        if let Some(weeks) = cal.and_then(|c| c.get("weeks")).and_then(Value::as_array) {
            for week in weeks {
                let Some(days) = week.get("contributionDays").and_then(Value::as_array) else {
                    continue;
                };
                for d in days {
                    calendar.push(ContributionDay {
                        date: str_at(d, "date").unwrap_or_default(),
                        count: d.get("contributionCount").and_then(Value::as_u64).unwrap_or(0) as u32,
                        color: str_at(d, "color").unwrap_or_else(|| "#161b22".into()),
                    });
                }
            }
        }

        let mut total_contributions = 0u64;
        for y in created_year.max(FIRST_YEAR)..=now_year {
            if let Some(n) = viewer
                .pointer(&format!("/y{y}/contributionCalendar/totalContributions"))
                .and_then(Value::as_u64)
            {
                total_contributions += n;
            }
        }
        total_contributions = total_contributions.max(last_year_contributions);

        let avatar_data_uri = avatar_url.as_deref().and_then(|u| self.fetch_data_uri(u).ok());

        Ok(ProfileStats {
            login,
            name,
            avatar_data_uri,
            created_year,
            total_contributions,
            last_year_contributions,
            calendar,
            fetched_at: Some(chrono::Utc::now().to_rfc3339()),
        })
    }

    /// Download an image and inline it as a base64 `data:` URI.
    fn fetch_data_uri(&self, url: &str) -> Result<String> {
        let resp = ureq::get(url)
            .set("User-Agent", UA)
            .call()
            .map_err(|e| AppError::Network(format!("avatar fetch: {e}")))?;
        let ctype = resp.header("content-type").unwrap_or("image/png").to_string();
        let mut bytes = Vec::new();
        resp.into_reader()
            .take(2_000_000) // avatars are tiny; cap defensively
            .read_to_end(&mut bytes)
            .map_err(|e| AppError::Network(e.to_string()))?;
        Ok(format!("data:{ctype};base64,{}", base64_encode(&bytes)))
    }
}

fn str_at(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(String::from)
}

fn current_year() -> i32 {
    use chrono::Datelike;
    chrono::Utc::now().year()
}

/// Minimal standard base64 (no padding shortcuts), to avoid a new dependency.
fn base64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { T[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(n & 63) as usize] as char } else { '=' });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::base64_encode;

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
