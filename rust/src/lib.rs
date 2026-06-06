//! `crowdsource` — the official first-party client for the crowdsource platform.
//!
//! One Rust core; Python, JS/wasm, and CLI surfaces build on top of it. This
//! crate holds the wire [`models`] (mirroring `crowdsource-server`'s `core::`),
//! the [`error`] types (RFC 7807), and — behind the default `client` feature —
//! an async [`Client`].
//!
//! ```no_run
//! # async fn run() -> Result<(), crowdsource::CrowdsourceError> {
//! use crowdsource::{Client, CompetitionQuery};
//! let client = Client::from_env()?;
//! let open = client.list_competitions(&CompetitionQuery::default()).await?;
//! println!("{} open competitions", open.total);
//! # Ok(()) }
//! ```

pub mod error;
pub mod models;

#[cfg(feature = "client")]
pub mod client;

pub use error::{CrowdsourceError, ProblemDetails};
pub use models::*;

#[cfg(feature = "client")]
pub use client::Client;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_serialize_to_server_snake_case() {
        assert_eq!(
            serde_json::to_string(&BountyMode::TopNEqual).unwrap(),
            "\"top_n_equal\""
        );
        assert_eq!(
            serde_json::to_string(&BountyMode::TopNWeighted).unwrap(),
            "\"top_n_weighted\""
        );
        assert_eq!(
            serde_json::to_string(&CompetitionMetric::F1Macro).unwrap(),
            "\"f1_macro\""
        );
        assert_eq!(
            serde_json::to_string(&CompetitionMetric::R2).unwrap(),
            "\"r2\""
        );
        assert_eq!(
            serde_json::to_string(&CompetitionStatus::ScoringFailed).unwrap(),
            "\"scoring_failed\""
        );
        assert_eq!(
            serde_json::to_string(&CompetitionType::Classification).unwrap(),
            "\"classification\""
        );
    }

    #[test]
    fn competition_deserializes_from_server_json() {
        let raw = serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000001",
            "org_id": "00000000-0000-0000-0000-000000000002",
            "title": "BTC close",
            "description": "Predict today's BTC close.",
            "competition_type": "regression",
            "metric": "rmse",
            "status": "open",
            "bounty_amount": 1000,
            "bounty_mode": "top_n_weighted",
            "bounty_top_n": 3,
            "bounty_weights": [50, 30, 20],
            "dataset_schema": null,
            "end_date": "2026-06-06T17:00:00Z",
            "created_at": "2026-06-06T09:00:00Z",
            "updated_at": "2026-06-06T09:00:00Z"
        });
        let c: Competition = serde_json::from_value(raw).unwrap();
        assert_eq!(c.metric, CompetitionMetric::Rmse);
        assert_eq!(c.bounty_mode, BountyMode::TopNWeighted);
        assert_eq!(c.status, CompetitionStatus::Open);
        assert_eq!(c.bounty_top_n, 3);
    }

    #[test]
    fn problem_details_renders_detail() {
        let p: ProblemDetails = serde_json::from_value(serde_json::json!({
            "type": "https://httpstatuses.io/402",
            "title": "Payment Required",
            "status": 402,
            "detail": "insufficient credits: need 1100, have 50"
        }))
        .unwrap();
        assert!(p.to_string().contains("insufficient credits"));
        assert!(p.to_string().contains("402"));
    }
}
