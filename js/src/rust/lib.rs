//! Browser (wasm) binding for the crowdsource client.
//!
//! Thin wasm-bindgen shell over `crowdsource::Client` (the Rust core). The core
//! uses reqwest's browser-fetch backend on wasm, so the transport, retries, and
//! types all live in Rust — this layer only marshals to/from JS values and turns
//! the async methods into Promises.

use crowdsource::{
    Client as CoreClient, CompetitionQuery, CompetitionStatus, CompetitionType, CreateCompetition,
    CreateDataSource, CreateSubmission, UpdateMe,
};
use js_sys::Promise;
use uuid::Uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

/// A crowdsource API client for the browser.
///
/// Every method returns a `Promise` that resolves to a plain JS object (or
/// array), or rejects with an `Error` whose message carries the server's
/// RFC 7807 `detail`.
#[wasm_bindgen]
pub struct Client {
    inner: CoreClient,
}

#[wasm_bindgen]
impl Client {
    /// `new Client(baseUrl, apiKey?)` — e.g. `new Client("https://api.crowdsource.sh")`.
    #[wasm_bindgen(constructor)]
    pub fn new(base_url: String, api_key: Option<String>) -> Result<Client, JsError> {
        let inner = CoreClient::new(base_url, api_key).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Client { inner })
    }

    /// `Client.withBearer(baseUrl, token)` — authenticate with a session JWT
    /// (e.g. the Supabase access token). This is what the browser app uses.
    #[wasm_bindgen(js_name = withBearer)]
    pub fn with_bearer(base_url: String, bearer_token: String) -> Result<Client, JsError> {
        let inner = CoreClient::with_bearer(base_url, bearer_token)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(Client { inner })
    }

    // ---- platform / config ----

    /// `summary()` — platform-wide stats.
    #[wasm_bindgen(js_name = summary)]
    pub fn summary(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.summary().await.map_err(err)?) })
    }

    /// `events(limit?)` — recent activity feed (ticker).
    #[wasm_bindgen(js_name = events)]
    pub fn events(&self, limit: Option<f64>) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            to_js(&inner.events(limit.map(|n| n as i64)).await.map_err(err)?)
        })
    }

    /// `economicConfig()` — active economic config + version.
    #[wasm_bindgen(js_name = economicConfig)]
    pub fn economic_config(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.economic_config().await.map_err(err)?) })
    }

    // ---- identity ----

    /// `me()` — the authenticated user.
    #[wasm_bindgen(js_name = me)]
    pub fn me(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.me().await.map_err(err)?) })
    }

    /// `updateMe(patch)` — `patch` is `{ display_name?, avatar_url? }`.
    #[wasm_bindgen(js_name = updateMe)]
    pub fn update_me(&self, patch: JsValue) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let patch: UpdateMe =
                serde_wasm_bindgen::from_value(patch).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.update_me(&patch).await.map_err(err)?)
        })
    }

    /// `creditBalance()`.
    #[wasm_bindgen(js_name = creditBalance)]
    pub fn credit_balance(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.credit_balance().await.map_err(err)?) })
    }

    /// `getOrg(id)`.
    #[wasm_bindgen(js_name = getOrg)]
    pub fn get_org(&self, id: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let id = Uuid::parse_str(&id).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.get_org(id).await.map_err(err)?)
        })
    }

    // ---- competitions ----

    /// `listCompetitions(status?, type?, limit?, offset?, mine?, hosted?, tag?)`.
    #[wasm_bindgen(js_name = listCompetitions)]
    #[allow(clippy::too_many_arguments)]
    pub fn list_competitions(
        &self,
        status: Option<String>,
        competition_type: Option<String>,
        limit: Option<f64>,
        offset: Option<f64>,
        mine: Option<bool>,
        hosted: Option<bool>,
        tag: Option<String>,
    ) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let query = CompetitionQuery {
                status: status.and_then(|s| parse_enum::<CompetitionStatus>(&s)),
                competition_type: competition_type.and_then(|s| parse_enum::<CompetitionType>(&s)),
                limit: limit.map(|n| n as i64),
                offset: offset.map(|n| n as i64),
                mine,
                hosted,
                tag,
            };
            to_js(&inner.list_competitions(&query).await.map_err(err)?)
        })
    }

    /// `getCompetition(id)`.
    #[wasm_bindgen(js_name = getCompetition)]
    pub fn get_competition(&self, id: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let id = Uuid::parse_str(&id).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.get_competition(id).await.map_err(err)?)
        })
    }

    /// `createCompetition(req)` — `req` is a `CreateCompetition` object.
    #[wasm_bindgen(js_name = createCompetition)]
    pub fn create_competition(&self, req: JsValue) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let req: CreateCompetition =
                serde_wasm_bindgen::from_value(req).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.create_competition(&req).await.map_err(err)?)
        })
    }

    /// `publishCompetition(id)` — move a draft to open.
    #[wasm_bindgen(js_name = publishCompetition)]
    pub fn publish_competition(&self, id: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let id = Uuid::parse_str(&id).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.publish_competition(id).await.map_err(err)?)
        })
    }

    /// `closeCompetition(id)` — close submissions early.
    #[wasm_bindgen(js_name = closeCompetition)]
    pub fn close_competition(&self, id: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let id = Uuid::parse_str(&id).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.close_competition(id).await.map_err(err)?)
        })
    }

    /// `leaderboard(id)`.
    #[wasm_bindgen(js_name = leaderboard)]
    pub fn leaderboard(&self, id: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let id = Uuid::parse_str(&id).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.leaderboard(id).await.map_err(err)?)
        })
    }

    // ---- predictions / submissions ----

    /// `submit(competitionId, body)` — `body` is `{ s3_key }` or `{ payload }`.
    #[wasm_bindgen(js_name = submit)]
    pub fn submit(&self, competition_id: String, body: JsValue) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let cid = Uuid::parse_str(&competition_id).map_err(|e| error(&e.to_string()))?;
            let body: CreateSubmission =
                serde_wasm_bindgen::from_value(body).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.submit(cid, &body).await.map_err(err)?)
        })
    }

    /// `listSubmissions(competitionId)`.
    #[wasm_bindgen(js_name = listSubmissions)]
    pub fn list_submissions(&self, competition_id: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let cid = Uuid::parse_str(&competition_id).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.list_submissions(cid).await.map_err(err)?)
        })
    }

    /// `listMySubmissions()`.
    #[wasm_bindgen(js_name = listMySubmissions)]
    pub fn list_my_submissions(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.list_my_submissions().await.map_err(err)?) })
    }

    // ---- api keys ----

    /// `listApiKeys()`.
    #[wasm_bindgen(js_name = listApiKeys)]
    pub fn list_api_keys(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.list_api_keys().await.map_err(err)?) })
    }

    /// `createApiKey(name)` — the returned `secret` is shown only once.
    #[wasm_bindgen(js_name = createApiKey)]
    pub fn create_api_key(&self, name: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.create_api_key(name).await.map_err(err)?) })
    }

    /// `deleteApiKey(id)` — revoke a key. Resolves to `undefined`.
    #[wasm_bindgen(js_name = deleteApiKey)]
    pub fn delete_api_key(&self, id: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let id = Uuid::parse_str(&id).map_err(|e| error(&e.to_string()))?;
            inner.delete_api_key(id).await.map_err(err)?;
            Ok(JsValue::UNDEFINED)
        })
    }

    // ---- data sources ----

    /// `listDataSources()`.
    #[wasm_bindgen(js_name = listDataSources)]
    pub fn list_data_sources(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.list_data_sources().await.map_err(err)?) })
    }

    /// `createDataSource(req)` — `req` is a `CreateDataSource` object.
    #[wasm_bindgen(js_name = createDataSource)]
    pub fn create_data_source(&self, req: JsValue) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let req: CreateDataSource =
                serde_wasm_bindgen::from_value(req).map_err(|e| error(&e.to_string()))?;
            to_js(&inner.create_data_source(&req).await.map_err(err)?)
        })
    }

    // ---- rank ----

    /// `rankUp()` — spend credits to advance one rank.
    #[wasm_bindgen(js_name = rankUp)]
    pub fn rank_up(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.rank_up().await.map_err(err)?) })
    }

    /// `rankDown()` — step down one rank (partial refund).
    #[wasm_bindgen(js_name = rankDown)]
    pub fn rank_down(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.rank_down().await.map_err(err)?) })
    }

    // ---- credits / checkout ----

    /// `createCheckout(amountCents)` — start a Stripe Checkout session; resolves
    /// to `{ checkout_url }`.
    #[wasm_bindgen(js_name = createCheckout)]
    pub fn create_checkout(&self, amount_cents: f64) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            to_js(&inner.create_checkout(amount_cents as i64).await.map_err(err)?)
        })
    }
}

fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

fn to_js<T: serde::Serialize>(v: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(v).map_err(|e| error(&e.to_string()))
}

fn err(e: crowdsource::CrowdsourceError) -> JsValue {
    error(&e.to_string())
}

fn error(msg: &str) -> JsValue {
    js_sys::Error::new(msg).into()
}
