//! HTTP client for the crowdsource API.
//!
//! Thin async wrapper over `reqwest`. Focused on the competition and prediction
//! flows the site and CLI need first. Paths target the server's current `/v1/`
//! surface; when the server moves to `/api/v1/` (server roadmap Phase 2.5),
//! update [`API_V1`].

use crate::error::{CrowdsourceError, ProblemDetails};
use crate::models::{
    AccessRow, ApiKey, CheckoutRequest, CheckoutResponse, Competition, CompetitionIndex,
    CompetitionListResponse, CompetitionQuery, CreateApiKey, CreateApiKeyResponse,
    CreateCompetition, CreateDataSource, CreateSubmission, CreditBalance, DataSource,
    EconomicConfigResponse, EventsResponse, GiftResponse, LeaderboardResponse, Me, Org,
    PublicProfile, RankTransition, RetractSubmission, Submission, Summary, UpdateMe,
};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde::Serialize;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use uuid::Uuid;

const DEFAULT_BASE_URL: &str = "https://api.crowdsource.sh";
/// The versioned API prefix on the live server. (Will become `/api/v1` later.)
const API_V1: &str = "/v1";
/// Max attempts (initial + retries) for idempotent GET requests.
#[cfg(not(target_arch = "wasm32"))]
const MAX_ATTEMPTS: u32 = 3;

/// A connected crowdsource API client.
#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    api_key: Option<String>,
    bearer: Option<String>,
    http: reqwest::Client,
}

impl Client {
    /// Build a client for `base_url` (e.g. `https://api.crowdsource.sh`) with an
    /// optional API key (`cs_pub_…` or `cs_sk_…`, sent as `X-API-Key`).
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
    ) -> Result<Self, CrowdsourceError> {
        Ok(Self {
            base_url: normalize_base(base_url),
            api_key,
            bearer: None,
            http: http_client()?,
        })
    }

    /// Build a client that authenticates with a bearer token (e.g. a Supabase
    /// session JWT), sent as `Authorization: Bearer …`. This is how browser
    /// sessions authenticate (the server accepts both Bearer and `X-API-Key`).
    pub fn with_bearer(
        base_url: impl Into<String>,
        bearer_token: impl Into<String>,
    ) -> Result<Self, CrowdsourceError> {
        Ok(Self {
            base_url: normalize_base(base_url),
            api_key: None,
            bearer: Some(bearer_token.into()),
            http: http_client()?,
        })
    }

    /// Build a client from the environment:
    /// `CROWDSOURCE_SERVER_URL` (default `https://api.crowdsource.sh`) and
    /// `CROWDSOURCE_API_KEY` (optional).
    pub fn from_env() -> Result<Self, CrowdsourceError> {
        let base = std::env::var("CROWDSOURCE_SERVER_URL")
            .unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let key = std::env::var("CROWDSOURCE_API_KEY").ok();
        Self::new(base, key)
    }

    fn build(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let mut req = self
            .http
            .request(method, format!("{}{}", self.base_url, path));
        // Bearer (session JWT) takes precedence; otherwise the API key.
        if let Some(token) = &self.bearer {
            req = req.bearer_auth(token);
        } else if let Some(key) = &self.api_key {
            req = req.header("X-API-Key", key);
        }
        req
    }

    /// Execute a request once (no retries). Used for non-idempotent writes —
    /// retrying a POST that may have spent credits is unsafe.
    async fn exec<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, CrowdsourceError> {
        let res = req.send().await?;
        let status = res.status();
        let bytes = res.bytes().await?;
        parse_response(status, &bytes)
    }

    /// Execute an idempotent GET, retrying transient failures (transport errors,
    /// 5xx, 429) with exponential backoff that honors `Retry-After`. On wasm
    /// (no async sleep primitive) this is a single attempt.
    #[cfg(not(target_arch = "wasm32"))]
    async fn exec_get<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, CrowdsourceError> {
        let mut attempt = 1u32;
        loop {
            // Clone so the original survives for the next attempt; a non-cloneable
            // body (never the case for GETs) falls back to a single send.
            let Some(this) = req.try_clone() else {
                return self.exec(req).await;
            };
            match this.send().await {
                Ok(res) => {
                    let status = res.status();
                    let retryable = status.is_server_error() || status.as_u16() == 429;
                    if retryable && attempt < MAX_ATTEMPTS {
                        let delay = retry_after(&res).unwrap_or_else(|| backoff(attempt));
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    let bytes = res.bytes().await?;
                    return parse_response(status, &bytes);
                }
                Err(e) => {
                    if attempt < MAX_ATTEMPTS
                        && (e.is_timeout() || e.is_connect() || e.is_request())
                    {
                        tokio::time::sleep(backoff(attempt)).await;
                        attempt += 1;
                        continue;
                    }
                    return Err(e.into());
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    async fn exec_get<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, CrowdsourceError> {
        self.exec(req).await
    }

    /// Execute a write whose success carries no (or an ignorable) body; maps a
    /// non-2xx into the RFC 7807 error.
    async fn exec_ok(&self, req: reqwest::RequestBuilder) -> Result<(), CrowdsourceError> {
        let res = req.send().await?;
        let status = res.status();
        if status.is_success() {
            return Ok(());
        }
        let bytes = res.bytes().await?;
        parse_response::<serde_json::Value>(status, &bytes).map(|_| ())
    }

    /// Execute a GET returning the raw text body (e.g. a CSV template).
    async fn exec_text(&self, req: reqwest::RequestBuilder) -> Result<String, CrowdsourceError> {
        let res = req.send().await?;
        let status = res.status();
        let bytes = res.bytes().await?;
        if status.is_success() {
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        } else {
            parse_response::<serde_json::Value>(status, &bytes).map(|_| String::new())
        }
    }

    // ---- health / identity ----

    /// `GET /health` — liveness probe. Returns the raw JSON body.
    pub async fn health(&self) -> Result<serde_json::Value, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, "/health")).await
    }

    /// `GET /v1/version` — build + connectivity info.
    pub async fn version(&self) -> Result<serde_json::Value, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/version")))
            .await
    }

    /// `GET /v1/summary` — platform-wide stats.
    pub async fn summary(&self) -> Result<Summary, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/summary")))
            .await
    }

    /// `GET /v1/events` — recent activity feed (ticker). `limit` is clamped
    /// server-side to `[1, 50]` (default 20).
    pub async fn events(&self, limit: Option<i64>) -> Result<EventsResponse, CrowdsourceError> {
        let mut req = self.build(Method::GET, &format!("{API_V1}/events"));
        if let Some(l) = limit {
            req = req.query(&[("limit", l.to_string())]);
        }
        self.exec_get(req).await
    }

    /// `GET /v1/config/economics` — the active economic config + its version.
    pub async fn economic_config(&self) -> Result<EconomicConfigResponse, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/config/economics")))
            .await
    }

    /// `GET /v1/me` — the authenticated user.
    pub async fn me(&self) -> Result<Me, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/me")))
            .await
    }

    /// `PATCH /v1/me` — update the caller's profile (display name, avatar).
    pub async fn update_me(&self, req: &UpdateMe) -> Result<Me, CrowdsourceError> {
        self.exec(self.build(Method::PATCH, &format!("{API_V1}/me")).json(req))
            .await
    }

    /// `GET /v1/me/credits` — credit balance.
    pub async fn credit_balance(&self) -> Result<CreditBalance, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/me/credits")))
            .await
    }

    /// `GET /v1/users/{handle}` — a user's public profile (no auth required).
    pub async fn profile(&self, handle: &str) -> Result<PublicProfile, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/users/{handle}")))
            .await
    }

    /// `POST /v1/competitions/{id}/access/request` — request access to a
    /// restricted competition (the host then approves or denies).
    pub async fn request_access(&self, competition_id: Uuid) -> Result<(), CrowdsourceError> {
        self.exec_ok(self.build(
            Method::POST,
            &format!("{API_V1}/competitions/{competition_id}/access/request"),
        ))
        .await
    }

    /// `POST /v1/competitions/{id}/access` — host: `invite` | `approve` | `deny`
    /// a user by handle.
    pub async fn manage_access(
        &self,
        competition_id: Uuid,
        handle: &str,
        action: &str,
    ) -> Result<(), CrowdsourceError> {
        let body = serde_json::json!({ "handle": handle, "action": action });
        self.exec_ok(
            self.build(
                Method::POST,
                &format!("{API_V1}/competitions/{competition_id}/access"),
            )
            .json(&body),
        )
        .await
    }

    /// `GET /v1/competitions/{id}/access` — host: list access requests + grants.
    pub async fn list_access(
        &self,
        competition_id: Uuid,
    ) -> Result<Vec<AccessRow>, CrowdsourceError> {
        self.exec_get(self.build(
            Method::GET,
            &format!("{API_V1}/competitions/{competition_id}/access"),
        ))
        .await
    }

    /// `POST /v1/credits/gift` — gift credits to another user by handle.
    pub async fn gift_credits(
        &self,
        recipient_handle: &str,
        amount: i64,
        message: Option<&str>,
    ) -> Result<GiftResponse, CrowdsourceError> {
        let body = serde_json::json!({
            "recipient_handle": recipient_handle,
            "amount": amount,
            "message": message,
        });
        self.exec(
            self.build(Method::POST, &format!("{API_V1}/credits/gift"))
                .json(&body),
        )
        .await
    }

    /// `GET /v1/orgs/:id`.
    pub async fn get_org(&self, org_id: Uuid) -> Result<Org, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/orgs/{org_id}")))
            .await
    }

    // ---- competitions ----

    /// `GET /v1/competitions` — list with optional filters.
    pub async fn list_competitions(
        &self,
        query: &CompetitionQuery,
    ) -> Result<CompetitionListResponse, CrowdsourceError> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(s) = query.status {
            params.push(("status", enum_str(&s)));
        }
        if let Some(t) = query.competition_type {
            params.push(("type", enum_str(&t)));
        }
        if let Some(l) = query.limit {
            params.push(("limit", l.to_string()));
        }
        if let Some(o) = query.offset {
            params.push(("offset", o.to_string()));
        }
        if query.mine == Some(true) {
            params.push(("mine", "true".to_string()));
        }
        if query.hosted == Some(true) {
            params.push(("hosted", "true".to_string()));
        }
        if let Some(tag) = &query.tag {
            params.push(("tag", tag.clone()));
        }
        if query.needs_resolution == Some(true) {
            params.push(("needs_resolution", "true".to_string()));
        }
        if let Some(sort) = &query.sort {
            params.push(("sort", sort.clone()));
        }
        let req = self
            .build(Method::GET, &format!("{API_V1}/competitions"))
            .query(&params);
        self.exec_get(req).await
    }

    /// `GET /v1/competitions/:id`.
    pub async fn get_competition(&self, id: Uuid) -> Result<Competition, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/competitions/{id}")))
            .await
    }

    /// `POST /v1/competitions` — create (deducts the creation fee + bounty).
    pub async fn create_competition(
        &self,
        req: &CreateCompetition,
    ) -> Result<Competition, CrowdsourceError> {
        self.exec(
            self.build(Method::POST, &format!("{API_V1}/competitions"))
                .json(req),
        )
        .await
    }

    /// `POST /v1/competitions/:id/publish` — move a draft to open.
    pub async fn publish_competition(&self, id: Uuid) -> Result<Competition, CrowdsourceError> {
        self.exec(self.build(Method::POST, &format!("{API_V1}/competitions/{id}/publish")))
            .await
    }

    /// `POST /v1/competitions/:id/close` — close submissions early.
    pub async fn close_competition(&self, id: Uuid) -> Result<Competition, CrowdsourceError> {
        self.exec(self.build(Method::POST, &format!("{API_V1}/competitions/{id}/close")))
            .await
    }

    /// `GET /v1/competitions/:id/leaderboard`.
    pub async fn leaderboard(&self, id: Uuid) -> Result<LeaderboardResponse, CrowdsourceError> {
        self.exec_get(self.build(
            Method::GET,
            &format!("{API_V1}/competitions/{id}/leaderboard"),
        ))
        .await
    }

    /// `GET /v1/competitions/:id/input-source` — the public input data source
    /// participants predict on (the resolution source is never exposed here).
    pub async fn input_source(&self, id: Uuid) -> Result<DataSource, CrowdsourceError> {
        self.exec_get(self.build(
            Method::GET,
            &format!("{API_V1}/competitions/{id}/input-source"),
        ))
        .await
    }

    /// `GET /v1/competitions/:id/index` — the row keys to predict + target shape.
    /// `dynamic` indices are fetched live from the input source server-side.
    pub async fn competition_index(&self, id: Uuid) -> Result<CompetitionIndex, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/competitions/{id}/index")))
            .await
    }

    /// `GET /v1/competitions/:id/index?format=csv` — a `key,value` CSV template
    /// pre-filled with the current keys (blank values) for bulk submission.
    pub async fn competition_index_template(&self, id: Uuid) -> Result<String, CrowdsourceError> {
        self.exec_text(
            self.build(Method::GET, &format!("{API_V1}/competitions/{id}/index"))
                .query(&[("format", "csv")]),
        )
        .await
    }

    // ---- datasets / resolution (tabular) ----

    /// `POST /v1/datasets/infer-schema` — infer a dataset spec from an uploaded
    /// file (`(filename, bytes)`) or a `url`. Returns the raw inference response
    /// (proposed spec, inferred columns, index/target candidates, index keys).
    pub async fn infer_schema(
        &self,
        file: Option<(String, Vec<u8>)>,
        url: Option<String>,
        format: Option<String>,
        auth_header: Option<String>,
    ) -> Result<serde_json::Value, CrowdsourceError> {
        let mut parts = Vec::new();
        if let Some((name, bytes)) = file {
            parts.push(MultipartPart::File("file".into(), name, bytes));
        }
        if let Some(u) = url {
            parts.push(MultipartPart::Field("url".into(), u));
        }
        if let Some(f) = format {
            parts.push(MultipartPart::Field("format".into(), f));
        }
        if let Some(a) = auth_header {
            parts.push(MultipartPart::Field("auth_header".into(), a));
        }
        let (content_type, body) = build_multipart(&parts);
        self.exec(
            self.build(Method::POST, &format!("{API_V1}/datasets/infer-schema"))
                .header("Content-Type", content_type)
                .body(body),
        )
        .await
    }

    /// `POST /v1/competitions/:id/resolution-file` — manually resolve a closed
    /// competition by uploading a results file. `index_column`/`target_column`/
    /// `format` override the dataset spec's defaults. The file is parsed + scored
    /// server-side and discarded.
    pub async fn resolution_file(
        &self,
        id: Uuid,
        filename: String,
        bytes: Vec<u8>,
        index_column: Option<String>,
        target_column: Option<String>,
        format: Option<String>,
    ) -> Result<(), CrowdsourceError> {
        let mut parts = vec![MultipartPart::File("file".into(), filename, bytes)];
        if let Some(c) = index_column {
            parts.push(MultipartPart::Field("index_column".into(), c));
        }
        if let Some(c) = target_column {
            parts.push(MultipartPart::Field("target_column".into(), c));
        }
        if let Some(f) = format {
            parts.push(MultipartPart::Field("format".into(), f));
        }
        let (content_type, body) = build_multipart(&parts);
        self.exec_ok(
            self.build(
                Method::POST,
                &format!("{API_V1}/competitions/{id}/resolution-file"),
            )
            .header("Content-Type", content_type)
            .body(body),
        )
        .await
    }

    // ---- predictions / submissions ----

    /// `POST /v1/competitions/:id/submissions` — submit a prediction. Build the
    /// body with [`CreateSubmission::from_payload`] (inline JSON) or
    /// [`CreateSubmission::from_s3_key`].
    pub async fn submit(
        &self,
        competition_id: Uuid,
        req: &CreateSubmission,
    ) -> Result<Submission, CrowdsourceError> {
        self.exec(
            self.build(
                Method::POST,
                &format!("{API_V1}/competitions/{competition_id}/submissions"),
            )
            .json(req),
        )
        .await
    }

    /// `GET /v1/competitions/:id/submissions`.
    pub async fn list_submissions(
        &self,
        competition_id: Uuid,
    ) -> Result<Vec<Submission>, CrowdsourceError> {
        self.exec_get(self.build(
            Method::GET,
            &format!("{API_V1}/competitions/{competition_id}/submissions"),
        ))
        .await
    }

    /// `GET /v1/me/submissions` — the caller's submissions.
    pub async fn list_my_submissions(&self) -> Result<Vec<Submission>, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/me/submissions")))
            .await
    }

    /// `POST /v1/competitions/:id/submissions/retract` — withdraw the caller's
    /// entry from an open competition, refunding the submission fee. The user is
    /// then blocked from resubmitting to that competition (iteration).
    pub async fn retract_submission(
        &self,
        competition_id: Uuid,
    ) -> Result<RetractSubmission, CrowdsourceError> {
        self.exec(self.build(
            Method::POST,
            &format!("{API_V1}/competitions/{competition_id}/submissions/retract"),
        ))
        .await
    }

    // ---- api keys ----

    /// `GET /v1/api-keys` — list the caller's API keys (secrets never returned).
    pub async fn list_api_keys(&self) -> Result<Vec<ApiKey>, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/api-keys")))
            .await
    }

    /// `POST /v1/api-keys` — create a key. The plaintext `secret` is returned
    /// exactly once in the response; store it immediately.
    pub async fn create_api_key(
        &self,
        name: impl Into<String>,
    ) -> Result<CreateApiKeyResponse, CrowdsourceError> {
        let body = CreateApiKey { name: name.into() };
        self.exec(
            self.build(Method::POST, &format!("{API_V1}/api-keys"))
                .json(&body),
        )
        .await
    }

    /// `DELETE /v1/api-keys/:id` — revoke a key.
    pub async fn delete_api_key(&self, id: Uuid) -> Result<(), CrowdsourceError> {
        let res = self
            .build(Method::DELETE, &format!("{API_V1}/api-keys/{id}"))
            .send()
            .await?;
        let status = res.status();
        if status.is_success() {
            Ok(())
        } else {
            let bytes = res.bytes().await?;
            parse_response::<serde_json::Value>(status, &bytes).map(|_| ())
        }
    }

    // ---- data sources ----

    /// `GET /v1/data-sources`.
    pub async fn list_data_sources(&self) -> Result<Vec<DataSource>, CrowdsourceError> {
        self.exec_get(self.build(Method::GET, &format!("{API_V1}/data-sources")))
            .await
    }

    /// `POST /v1/data-sources` — register a data source.
    pub async fn create_data_source(
        &self,
        req: &CreateDataSource,
    ) -> Result<DataSource, CrowdsourceError> {
        self.exec(
            self.build(Method::POST, &format!("{API_V1}/data-sources"))
                .json(req),
        )
        .await
    }

    // ---- rank ----

    /// `POST /v1/me/rank/up` — spend credits to advance one rank.
    pub async fn rank_up(&self) -> Result<RankTransition, CrowdsourceError> {
        self.exec(self.build(Method::POST, &format!("{API_V1}/me/rank/up")))
            .await
    }

    /// `POST /v1/me/rank/down` — step down one rank (partial refund).
    pub async fn rank_down(&self) -> Result<RankTransition, CrowdsourceError> {
        self.exec(self.build(Method::POST, &format!("{API_V1}/me/rank/down")))
            .await
    }

    // ---- credits / checkout ----

    /// `POST /v1/credits/checkout` — start a Stripe Checkout session for the
    /// credit pack priced at `amount_cents`. Returns the URL to open.
    pub async fn create_checkout(
        &self,
        amount_cents: i64,
    ) -> Result<CheckoutResponse, CrowdsourceError> {
        let body = CheckoutRequest { amount_cents };
        self.exec(
            self.build(Method::POST, &format!("{API_V1}/credits/checkout"))
                .json(&body),
        )
        .await
    }
}

/// A `multipart/form-data` boundary unlikely to occur in field/file content.
const MP_BOUNDARY: &str = "----crowdsourceSdkBoundaryQ9z1XoQ9z1Xo";

/// One part of a multipart body: a text field or an uploaded file.
enum MultipartPart {
    Field(String, String),
    File(String, String, Vec<u8>),
}

/// Build a `multipart/form-data` body by hand (reqwest's `multipart` feature is
/// off, and this is portable across the native + browser-fetch backends).
/// Returns the `Content-Type` header value and the raw body bytes.
fn build_multipart(parts: &[MultipartPart]) -> (String, Vec<u8>) {
    let mut body: Vec<u8> = Vec::new();
    for part in parts {
        body.extend_from_slice(format!("--{MP_BOUNDARY}\r\n").as_bytes());
        match part {
            MultipartPart::Field(name, value) => {
                body.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
                );
                body.extend_from_slice(value.as_bytes());
            }
            MultipartPart::File(name, filename, bytes) => {
                body.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: application/octet-stream\r\n\r\n"
                    )
                    .as_bytes(),
                );
                body.extend_from_slice(bytes);
            }
        }
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(format!("--{MP_BOUNDARY}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={MP_BOUNDARY}"), body)
}

/// Turn a finished HTTP response (status + body bytes) into a typed result,
/// mapping non-2xx into a [`CrowdsourceError::Api`] from the RFC 7807 body.
fn parse_response<T: DeserializeOwned>(
    status: reqwest::StatusCode,
    bytes: &[u8],
) -> Result<T, CrowdsourceError> {
    if status.is_success() {
        Ok(serde_json::from_slice(bytes)?)
    } else {
        let problem =
            serde_json::from_slice::<ProblemDetails>(bytes).unwrap_or_else(|_| ProblemDetails {
                problem_type: None,
                title: Some(status.canonical_reason().unwrap_or("error").to_string()),
                status: status.as_u16(),
                detail: None,
            });
        Err(CrowdsourceError::Api(problem))
    }
}

/// Backoff for retry attempt `n` (1-based): 250ms, 500ms, …
#[cfg(not(target_arch = "wasm32"))]
fn backoff(attempt: u32) -> Duration {
    Duration::from_millis(250u64 << (attempt.saturating_sub(1)).min(4))
}

/// Parse a `Retry-After` header (delta-seconds form) into a delay.
#[cfg(not(target_arch = "wasm32"))]
fn retry_after(res: &reqwest::Response) -> Option<Duration> {
    res.headers()
        .get(reqwest::header::RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(Duration::from_secs)
}

/// Serialize a `serde(rename_all = "snake_case")` enum to its wire string.
fn enum_str<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|x| x.as_str().map(str::to_string))
        .unwrap_or_default()
}

fn normalize_base(base_url: impl Into<String>) -> String {
    base_url.into().trim_end_matches('/').to_string()
}

// The wasm build uses the browser fetch backend, whose ClientBuilder doesn't
// support timeout/user_agent — use the default client there.
fn http_client() -> Result<reqwest::Client, CrowdsourceError> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        Ok(reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("crowdsource-rs/", env!("CARGO_PKG_VERSION")))
            .build()?)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Ok(reqwest::Client::new())
    }
}
