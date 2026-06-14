//! Python binding for the crowdsource client.
//!
//! Thin pyo3 shell over `crowdsource::Client` (the Rust core). The core is async
//! (reqwest); each method blocks on an internal Tokio runtime so the Python API
//! is plain and synchronous. Results come back as native Python objects (dicts /
//! lists) via `pythonize`.

use crowdsource_core::{
    Client as CoreClient, CompetitionQuery, CompetitionStatus, CompetitionType, CreateCompetition,
    CreateDataSource, CreateSubmission, UpdateMe,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use uuid::Uuid;

#[pyclass]
struct Client {
    inner: CoreClient,
    rt: tokio::runtime::Runtime,
}

#[pymethods]
impl Client {
    /// `Client(base_url, api_key=None)`.
    #[new]
    #[pyo3(signature = (base_url, api_key=None))]
    fn new(base_url: String, api_key: Option<String>) -> PyResult<Self> {
        let inner = CoreClient::new(base_url, api_key).map_err(pyerr)?;
        Ok(Self {
            inner,
            rt: runtime()?,
        })
    }

    /// Build from `CROWDSOURCE_SERVER_URL` / `CROWDSOURCE_API_KEY`.
    #[staticmethod]
    fn from_env() -> PyResult<Self> {
        let inner = CoreClient::from_env().map_err(pyerr)?;
        Ok(Self {
            inner,
            rt: runtime()?,
        })
    }

    /// Authenticate with a bearer token (e.g. a Supabase session JWT).
    #[staticmethod]
    fn with_bearer(base_url: String, bearer_token: String) -> PyResult<Self> {
        let inner = CoreClient::with_bearer(base_url, bearer_token).map_err(pyerr)?;
        Ok(Self {
            inner,
            rt: runtime()?,
        })
    }

    // ---- platform / config ----

    fn summary(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self.rt.block_on(self.inner.summary()).map_err(pyerr)?;
        to_py(py, &res)
    }

    #[pyo3(signature = (limit=None))]
    fn events(&self, py: Python<'_>, limit: Option<i64>) -> PyResult<Py<PyAny>> {
        let res = self.rt.block_on(self.inner.events(limit)).map_err(pyerr)?;
        to_py(py, &res)
    }

    fn economic_config(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self
            .rt
            .block_on(self.inner.economic_config())
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    // ---- identity ----

    fn me(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self.rt.block_on(self.inner.me()).map_err(pyerr)?;
        to_py(py, &res)
    }

    fn update_me(&self, py: Python<'_>, patch: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let patch: UpdateMe =
            pythonize::depythonize(patch).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.update_me(&patch))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn credit_balance(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self
            .rt
            .block_on(self.inner.credit_balance())
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn get_org(&self, py: Python<'_>, org_id: String) -> PyResult<Py<PyAny>> {
        let id = Uuid::parse_str(&org_id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self.rt.block_on(self.inner.get_org(id)).map_err(pyerr)?;
        to_py(py, &res)
    }

    // ---- competitions ----

    #[pyo3(signature = (status=None, competition_type=None, limit=None, offset=None, mine=None, hosted=None, tag=None))]
    #[allow(clippy::too_many_arguments)]
    fn list_competitions(
        &self,
        py: Python<'_>,
        status: Option<String>,
        competition_type: Option<String>,
        limit: Option<i64>,
        offset: Option<i64>,
        mine: Option<bool>,
        hosted: Option<bool>,
        tag: Option<String>,
    ) -> PyResult<Py<PyAny>> {
        let query = CompetitionQuery {
            status: status.and_then(|s| parse_enum::<CompetitionStatus>(&s)),
            competition_type: competition_type.and_then(|s| parse_enum::<CompetitionType>(&s)),
            limit,
            offset,
            mine,
            hosted,
            tag,
        };
        let res = self
            .rt
            .block_on(self.inner.list_competitions(&query))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn get_competition(&self, py: Python<'_>, id: String) -> PyResult<Py<PyAny>> {
        let id = Uuid::parse_str(&id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.get_competition(id))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn create_competition(&self, py: Python<'_>, req: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let req: CreateCompetition =
            pythonize::depythonize(req).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.create_competition(&req))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn publish_competition(&self, py: Python<'_>, id: String) -> PyResult<Py<PyAny>> {
        let id = Uuid::parse_str(&id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.publish_competition(id))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn close_competition(&self, py: Python<'_>, id: String) -> PyResult<Py<PyAny>> {
        let id = Uuid::parse_str(&id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.close_competition(id))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn leaderboard(&self, py: Python<'_>, id: String) -> PyResult<Py<PyAny>> {
        let id = Uuid::parse_str(&id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.leaderboard(id))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    // ---- predictions / submissions ----

    /// `submit(competition_id, body)` — `body` is a dict: `{"payload": ...}` or
    /// `{"s3_key": "..."}`.
    fn submit(
        &self,
        py: Python<'_>,
        competition_id: String,
        body: &Bound<'_, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        let cid =
            Uuid::parse_str(&competition_id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let body: CreateSubmission =
            pythonize::depythonize(body).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.submit(cid, &body))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn list_submissions(&self, py: Python<'_>, competition_id: String) -> PyResult<Py<PyAny>> {
        let cid =
            Uuid::parse_str(&competition_id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.list_submissions(cid))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn list_my_submissions(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self
            .rt
            .block_on(self.inner.list_my_submissions())
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    // ---- api keys ----

    fn list_api_keys(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self
            .rt
            .block_on(self.inner.list_api_keys())
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    /// Create an API key. The returned `secret` is shown only once.
    fn create_api_key(&self, py: Python<'_>, name: String) -> PyResult<Py<PyAny>> {
        let res = self
            .rt
            .block_on(self.inner.create_api_key(name))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn delete_api_key(&self, id: String) -> PyResult<()> {
        let id = Uuid::parse_str(&id).map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.rt
            .block_on(self.inner.delete_api_key(id))
            .map_err(pyerr)
    }

    // ---- data sources ----

    fn list_data_sources(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self
            .rt
            .block_on(self.inner.list_data_sources())
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    fn create_data_source(&self, py: Python<'_>, req: &Bound<'_, PyAny>) -> PyResult<Py<PyAny>> {
        let req: CreateDataSource =
            pythonize::depythonize(req).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let res = self
            .rt
            .block_on(self.inner.create_data_source(&req))
            .map_err(pyerr)?;
        to_py(py, &res)
    }

    // ---- rank ----

    fn rank_up(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self.rt.block_on(self.inner.rank_up()).map_err(pyerr)?;
        to_py(py, &res)
    }

    fn rank_down(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let res = self.rt.block_on(self.inner.rank_down()).map_err(pyerr)?;
        to_py(py, &res)
    }

    // ---- credits / checkout ----

    fn create_checkout(&self, py: Python<'_>, amount_cents: i64) -> PyResult<Py<PyAny>> {
        let res = self
            .rt
            .block_on(self.inner.create_checkout(amount_cents))
            .map_err(pyerr)?;
        to_py(py, &res)
    }
}

fn runtime() -> PyResult<tokio::runtime::Runtime> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|e| PyValueError::new_err(e.to_string()))
}

fn parse_enum<T: serde::de::DeserializeOwned>(s: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(s.to_string())).ok()
}

fn to_py<T: serde::Serialize>(py: Python<'_>, v: &T) -> PyResult<Py<PyAny>> {
    Ok(pythonize::pythonize(py, v)
        .map_err(|e| PyValueError::new_err(e.to_string()))?
        .unbind())
}

fn pyerr(e: crowdsource_core::CrowdsourceError) -> PyErr {
    PyValueError::new_err(e.to_string())
}

#[pymodule]
fn crowdsource(_py: Python, m: &Bound<PyModule>) -> PyResult<()> {
    m.add_class::<Client>()?;
    Ok(())
}
