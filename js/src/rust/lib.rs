//! Browser (wasm) binding for the crowdsource client.
//!
//! Thin wasm-bindgen shell over `crowdsource::Client` (the Rust core). The core
//! uses reqwest's browser-fetch backend on wasm, so the transport, retries, and
//! types all live in Rust — this layer only marshals to/from JS values and turns
//! the async methods into Promises.

use crowdsource::{
    Client as CoreClient, CompetitionQuery, CompetitionStatus, CompetitionType, CreateCompetition,
    CreateSubmission,
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

    /// `listCompetitions(status?, type?, limit?, offset?, mine?)`.
    #[wasm_bindgen(js_name = listCompetitions)]
    pub fn list_competitions(
        &self,
        status: Option<String>,
        competition_type: Option<String>,
        limit: Option<f64>,
        offset: Option<f64>,
        mine: Option<bool>,
    ) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let query = CompetitionQuery {
                status: status.and_then(|s| parse_enum::<CompetitionStatus>(&s)),
                competition_type: competition_type.and_then(|s| parse_enum::<CompetitionType>(&s)),
                limit: limit.map(|n| n as i64),
                offset: offset.map(|n| n as i64),
                mine,
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

    /// `submit(competitionId, s3Key)` — submit a prediction.
    #[wasm_bindgen(js_name = submit)]
    pub fn submit(&self, competition_id: String, s3_key: String) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move {
            let cid = Uuid::parse_str(&competition_id).map_err(|e| error(&e.to_string()))?;
            to_js(&inner
                .submit(cid, &CreateSubmission { s3_key })
                .await
                .map_err(err)?)
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

    /// `me()` — the authenticated user.
    #[wasm_bindgen(js_name = me)]
    pub fn me(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.me().await.map_err(err)?) })
    }

    /// `creditBalance()`.
    #[wasm_bindgen(js_name = creditBalance)]
    pub fn credit_balance(&self) -> Promise {
        let inner = self.inner.clone();
        future_to_promise(async move { to_js(&inner.credit_balance().await.map_err(err)?) })
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
