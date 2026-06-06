//! Wire types for the crowdsource API.
//!
//! These mirror `crowdsource-server`'s `core::` structs and enums one-to-one.
//! All enums use `serde(rename_all = "snake_case")` to match the server's
//! `serde`/`sqlx` representation. Drift between these and the server is a bug.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionType {
    Classification,
    Regression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionMetric {
    Accuracy,
    F1Macro,
    Rmse,
    Mae,
    R2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionStatus {
    Draft,
    Open,
    Closed,
    Scoring,
    Scored,
    ScoringFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BountyMode {
    TopNEqual,
    TopNWeighted,
}

/// A competition as returned by `GET /v1/competitions/:id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Competition {
    pub id: Uuid,
    pub org_id: Uuid,
    pub title: String,
    pub description: String,
    pub competition_type: CompetitionType,
    pub metric: CompetitionMetric,
    pub status: CompetitionStatus,
    pub bounty_amount: i64,
    pub bounty_mode: BountyMode,
    pub bounty_top_n: i32,
    #[serde(default)]
    pub bounty_weights: Option<serde_json::Value>,
    #[serde(default)]
    pub dataset_schema: Option<serde_json::Value>,
    pub end_date: DateTime<Utc>,
    #[serde(default)]
    pub recurring_interval: Option<String>,
    #[serde(default)]
    pub input_source_id: Option<Uuid>,
    #[serde(default)]
    pub resolution_source_id: Option<Uuid>,
    #[serde(default)]
    pub resolution_offset_minutes: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Body for `POST /v1/competitions`. Optional fields are omitted when `None`.
#[derive(Debug, Clone, Serialize)]
pub struct CreateCompetition {
    pub title: String,
    pub description: String,
    pub competition_type: CompetitionType,
    pub metric: CompetitionMetric,
    pub bounty_amount: i64,
    pub bounty_mode: BountyMode,
    pub bounty_top_n: i32,
    pub end_date: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounty_weights: Option<Vec<i32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_schema: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oracle_auth_header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_offset_minutes: Option<i32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompetitionListResponse {
    pub competitions: Vec<Competition>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

/// Filters for `GET /v1/competitions`.
#[derive(Debug, Clone, Default)]
pub struct CompetitionQuery {
    pub status: Option<CompetitionStatus>,
    pub competition_type: Option<CompetitionType>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub mine: Option<bool>,
}

/// A prediction submission (`POST /v1/competitions/:id/submissions`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submission {
    pub id: Uuid,
    pub competition_id: Uuid,
    pub user_id: Uuid,
    pub s3_key: String,
    pub score: Option<f64>,
    pub rank: Option<i32>,
    pub payout: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub scored_at: Option<DateTime<Utc>>,
}

/// Body for creating a submission.
#[derive(Debug, Clone, Serialize)]
pub struct CreateSubmission {
    pub s3_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreditBalance {
    pub balance: i64,
    pub purchased_total: i64,
    pub earned_total: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Me {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub org_id: Uuid,
    pub rank_tier: String,
    pub rank_level: i32,
}
