//! HTTP client for the crowdsource API.
//!
//! Thin async wrapper over `reqwest`. Focused on the competition and prediction
//! flows the site and CLI need first. Paths target the server's current `/v1/`
//! surface; when the server moves to `/api/v1/` (server roadmap Phase 2.5),
//! update [`API_V1`].

use crate::error::{CrowdsourceError, ProblemDetails};
use crate::models::{
    Competition, CompetitionListResponse, CompetitionQuery, CreateCompetition, CreateSubmission,
    CreditBalance, Me, Submission,
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

/// A connected crowdsource API client.
#[derive(Clone, Debug)]
pub struct Client {
    base_url: String,
    api_key: Option<String>,
    http: reqwest::Client,
}

impl Client {
    /// Build a client for `base_url` (e.g. `https://api.crowdsource.sh`) with an
    /// optional API key (`cs_pub_…` or `cs_sk_…`, sent as `X-API-Key`).
    pub fn new(
        base_url: impl Into<String>,
        api_key: Option<String>,
    ) -> Result<Self, CrowdsourceError> {
        // The wasm build uses the browser fetch backend, whose ClientBuilder
        // doesn't support timeout/user_agent — use the default client there.
        #[cfg(not(target_arch = "wasm32"))]
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("crowdsource-rs/", env!("CARGO_PKG_VERSION")))
            .build()?;
        #[cfg(target_arch = "wasm32")]
        let http = reqwest::Client::new();

        let base_url = base_url.into().trim_end_matches('/').to_string();
        Ok(Self {
            base_url,
            api_key,
            http,
        })
    }

    /// Build a client from the environment:
    /// `CROWDSOURCE_SERVER_URL` (default `https://api.crowdsource.sh`) and
    /// `CROWDSOURCE_API_KEY` (optional).
    pub fn from_env() -> Result<Self, CrowdsourceError> {
        let base =
            std::env::var("CROWDSOURCE_SERVER_URL").unwrap_or_else(|_| DEFAULT_BASE_URL.to_string());
        let key = std::env::var("CROWDSOURCE_API_KEY").ok();
        Self::new(base, key)
    }

    fn build(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let mut req = self.http.request(method, format!("{}{}", self.base_url, path));
        if let Some(key) = &self.api_key {
            req = req.header("X-API-Key", key);
        }
        req
    }

    async fn exec<T: DeserializeOwned>(
        &self,
        req: reqwest::RequestBuilder,
    ) -> Result<T, CrowdsourceError> {
        let res = req.send().await?;
        let status = res.status();
        let bytes = res.bytes().await?;
        if status.is_success() {
            Ok(serde_json::from_slice(&bytes)?)
        } else {
            let problem = serde_json::from_slice::<ProblemDetails>(&bytes).unwrap_or_else(|_| {
                ProblemDetails {
                    problem_type: None,
                    title: Some(status.canonical_reason().unwrap_or("error").to_string()),
                    status: status.as_u16(),
                    detail: None,
                }
            });
            Err(CrowdsourceError::Api(problem))
        }
    }

    // ---- health / identity ----

    /// `GET /health` — liveness probe. Returns the raw JSON body.
    pub async fn health(&self) -> Result<serde_json::Value, CrowdsourceError> {
        self.exec(self.build(Method::GET, "/health")).await
    }

    /// `GET /v1/version` — build + connectivity info.
    pub async fn version(&self) -> Result<serde_json::Value, CrowdsourceError> {
        self.exec(self.build(Method::GET, &format!("{API_V1}/version")))
            .await
    }

    /// `GET /v1/me` — the authenticated user.
    pub async fn me(&self) -> Result<Me, CrowdsourceError> {
        self.exec(self.build(Method::GET, &format!("{API_V1}/me")))
            .await
    }

    /// `GET /v1/me/credits` — credit balance.
    pub async fn credit_balance(&self) -> Result<CreditBalance, CrowdsourceError> {
        self.exec(self.build(Method::GET, &format!("{API_V1}/me/credits")))
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
        let req = self
            .build(Method::GET, &format!("{API_V1}/competitions"))
            .query(&params);
        self.exec(req).await
    }

    /// `GET /v1/competitions/:id`.
    pub async fn get_competition(&self, id: Uuid) -> Result<Competition, CrowdsourceError> {
        self.exec(self.build(Method::GET, &format!("{API_V1}/competitions/{id}")))
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

    // ---- predictions / submissions ----

    /// `POST /v1/competitions/:id/submissions` — submit a prediction.
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
        self.exec(self.build(
            Method::GET,
            &format!("{API_V1}/competitions/{competition_id}/submissions"),
        ))
        .await
    }

    /// `GET /v1/me/submissions` — the caller's submissions.
    pub async fn list_my_submissions(&self) -> Result<Vec<Submission>, CrowdsourceError> {
        self.exec(self.build(Method::GET, &format!("{API_V1}/me/submissions")))
            .await
    }
}

/// Serialize a `serde(rename_all = "snake_case")` enum to its wire string.
fn enum_str<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v)
        .ok()
        .and_then(|x| x.as_str().map(str::to_string))
        .unwrap_or_default()
}
