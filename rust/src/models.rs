//! Wire types for the crowdsource API.
//!
//! These mirror `crowdsource-server`'s `core::` structs and enums one-to-one.
//! All enums use `serde(rename_all = "snake_case")` to match the server's
//! `serde`/`sqlx` representation. Drift between these and the server is a bug.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionType {
    #[default]
    Classification,
    Regression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CompetitionMetric {
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BountyMode {
    #[default]
    TopNEqual,
    TopNWeighted,
}

/// Rank tier. Ordered bronze → legend; `Bronze` is the default (new accounts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RankTier {
    #[default]
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
    Master,
    Legend,
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
    /// Minimum rank required to enter. Defaults to `Bronze`.
    #[serde(default)]
    pub min_rank: RankTier,
    /// Per-submission fee (credits), stamped from config at creation.
    #[serde(default)]
    pub submission_fee: i64,
    #[serde(default)]
    pub tags: Vec<String>,
    pub end_date: DateTime<Utc>,
    #[serde(default)]
    pub recurring_interval: Option<String>,
    #[serde(default)]
    pub recurring_close_time: Option<String>,
    #[serde(default)]
    pub recurring_timezone: Option<String>,
    #[serde(default)]
    pub input_source_id: Option<Uuid>,
    #[serde(default)]
    pub resolution_source_id: Option<Uuid>,
    #[serde(default)]
    pub resolution_offset_minutes: Option<i32>,
    #[serde(default)]
    pub min_participants: Option<i32>,
    #[serde(default)]
    pub min_score: Option<f64>,
    /// Join mode: `public` (default), `restricted`, or `invite`.
    #[serde(default = "default_public")]
    pub access_mode: String,
    /// Hidden from browse / get-by-id for non-owner/non-invited.
    #[serde(default)]
    pub unlisted: bool,
    /// The caller's access relationship to a non-public comp (owner/requested/
    /// approved/invited/denied), when authenticated.
    #[serde(default)]
    pub my_access: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_public() -> String {
    "public".to_string()
}

/// Body for `POST /v1/competitions`. Optional fields are omitted when `None`.
/// `Deserialize` lets the wasm/python bindings construct one from a JS/Python
/// object; `Default` enables the builder pattern (`CreateCompetition { title,
/// ..Default::default() }`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateCompetition {
    pub title: String,
    pub description: String,
    pub competition_type: CompetitionType,
    pub metric: CompetitionMetric,
    pub bounty_amount: i64,
    pub bounty_mode: BountyMode,
    pub bounty_top_n: i32,
    pub end_date: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounty_weights: Option<Vec<i32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_schema: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oracle_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oracle_auth_header: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_offset_minutes: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_rank: Option<RankTier>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_source_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution_source_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurring_interval: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurring_close_time: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recurring_timezone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_participants: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f64>,
    /// Join mode: `public` (default), `restricted`, or `invite`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_mode: Option<String>,
    /// Hide from browse / get-by-id (private). Only for non-public modes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unlisted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionListResponse {
    pub competitions: Vec<Competition>,
    pub total: i64,
}

/// Filters for `GET /v1/competitions`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct CompetitionQuery {
    pub status: Option<CompetitionStatus>,
    pub competition_type: Option<CompetitionType>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    /// `mine=true` — competitions the caller has submitted to.
    pub mine: Option<bool>,
    /// `hosted=true` — competitions owned by the caller's org.
    pub hosted: Option<bool>,
    /// Filter to a single tag.
    pub tag: Option<String>,
    /// `needs_resolution=true` — the caller org's closed, manual (no oracle URL),
    /// unresolved competitions (the resolution queue).
    pub needs_resolution: Option<bool>,
    /// Sort order: `created` (default), `ending`, `fee`, `bounty`.
    pub sort: Option<String>,
}

/// The submission index for a competition (`GET /v1/competitions/:id/index`):
/// the row keys participants predict, plus the target shape. For a `dynamic`
/// index the keys are fetched live from the input source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitionIndex {
    /// The dataset-spec index column (the key participants predict for).
    pub index_column: String,
    /// `fixed` (stored keys) or `dynamic` (fetched from the input source).
    pub mode: String,
    /// `number` (regression) or `class` (classification).
    pub target_kind: String,
    /// Allowed labels when `target_kind == "class"`.
    #[serde(default)]
    pub classes: Option<Vec<String>>,
    /// Number of keys in the index.
    pub count: i64,
    /// The current row keys (capped server-side).
    #[serde(default)]
    pub keys: Vec<String>,
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
    /// Set when the entry was voluntarily withdrawn (refunded, excluded from
    /// scoring, and not resubmittable). `#[serde(default)]` for forward-compat
    /// with servers that predate retraction.
    #[serde(default)]
    pub retracted_at: Option<DateTime<Utc>>,
}

/// Result of retracting a submission (`POST /v1/competitions/:id/submissions/retract`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetractSubmission {
    pub submission_id: String,
    /// Credits refunded (the per-submission fee).
    pub refunded: i64,
    /// The caller's resulting credit balance.
    pub balance: i64,
}

/// Body for creating a submission. Provide exactly one of `s3_key` (a key
/// previously uploaded to object storage) or `payload` (inline prediction JSON).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateSubmission {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s3_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
}

impl CreateSubmission {
    /// Submit by reference to an object-storage key.
    pub fn from_s3_key(s3_key: impl Into<String>) -> Self {
        Self {
            s3_key: Some(s3_key.into()),
            payload: None,
        }
    }

    /// Submit an inline prediction payload (the common SDK path).
    pub fn from_payload(payload: serde_json::Value) -> Self {
        Self {
            s3_key: None,
            payload: Some(payload),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditBalance {
    pub balance: i64,
    pub purchased_total: i64,
    pub earned_total: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Me {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub org_id: Uuid,
    pub rank_tier: String,
    pub rank_level: i32,
    #[serde(default)]
    pub is_admin: bool,
    #[serde(default)]
    pub created_at: Option<DateTime<Utc>>,
}

/// Body for `PATCH /v1/me`. Only the set fields are sent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateMe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
}

// ---- leaderboard ----

/// One row of a competition leaderboard (`GET /v1/competitions/:id/leaderboard`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    /// 1-based rank; `None` while the competition is still open.
    pub rank: Option<i32>,
    /// Anonymized handle (e.g. `player-xxxxxxxx`).
    pub handle: String,
    pub score: Option<f64>,
    pub payout: Option<i64>,
    /// True for the authenticated caller's own row.
    #[serde(default)]
    pub is_you: bool,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardResponse {
    pub competition_id: Uuid,
    pub status: CompetitionStatus,
    pub total: i64,
    pub entries: Vec<LeaderboardEntry>,
}

// ---- platform summary ----

/// Platform-wide stats (`GET /v1/summary`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub competitions_open: i64,
    pub competitions_total: i64,
    pub active_bounties: i64,
    pub predictions_today: i64,
    pub predictions_total: i64,
    pub predictors: i64,
    pub credits_paid: i64,
}

// ---- events / ticker ----

/// One activity-feed event (`GET /v1/events`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// `competition_opened` | `win` | `rank_up` | `submission`.
    pub kind: String,
    pub ts: DateTime<Utc>,
    pub handle: Option<String>,
    pub title: Option<String>,
    pub competition_id: Option<Uuid>,
    pub amount: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventsResponse {
    pub events: Vec<Event>,
}

// ---- API keys ----

/// An API key as listed by `GET /v1/api-keys` (never includes the secret).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKey {
    pub name: String,
}

/// Response to `POST /v1/api-keys`. `secret` is the plaintext key, returned
/// exactly once — store it now, it cannot be retrieved again.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub secret: String,
    pub created_at: DateTime<Utc>,
}

// ---- data sources ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSource {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub url: String,
    pub http_method: String,
    pub auth_header: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub compatible_types: Vec<String>,
    pub schema_format: Option<String>,
    pub schema_text: Option<String>,
    pub openapi_path: Option<String>,
    #[serde(default)]
    pub last_probe: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateDataSource {
    pub name: String,
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_header: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roles: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatible_types: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openapi_path: Option<String>,
}

// ---- rank ----

/// Result of `POST /v1/me/rank/up` or `/down`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankTransition {
    pub rank_tier: String,
    pub rank_level: i32,
    /// New credit balance after the transaction.
    pub balance: i64,
    /// Credit delta: negative for rank up (cost), positive for rank down (refund).
    pub delta: i64,
}

// ---- credits / checkout ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutRequest {
    /// Must equal an active credit pack's `price_cents`.
    pub amount_cents: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutResponse {
    pub checkout_url: String,
}

// ---- orgs ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Org {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

// ---- economic config ----

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditPack {
    pub price_cents: i64,
    pub credits_granted: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreationFees {
    pub classification: i64,
    pub regression: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelUpCost {
    pub bronze: i64,
    pub silver: i64,
    pub gold: i64,
    pub platinum: i64,
    pub diamond: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierEntryCost {
    pub silver: i64,
    pub gold: i64,
    pub platinum: i64,
    pub diamond: i64,
    pub master: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankEconomics {
    pub level_up_cost: LevelUpCost,
    pub tier_entry_cost: TierEntryCost,
    pub rank_down_refund_num: i64,
    pub rank_down_refund_den: i64,
}

/// A per-tier credit amount map (bronze → diamond).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierAmounts {
    pub bronze: i64,
    pub silver: i64,
    pub gold: i64,
    pub platinum: i64,
    pub diamond: i64,
}

/// Per-capability rollout gates. `true` = the action is enabled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gates {
    pub registration: bool,
    pub api_keys: bool,
    pub submissions: bool,
    pub competitions: bool,
    pub rank_up: bool,
    pub buy_credits: bool,
    pub data_sources: bool,
    pub datasets: bool,
    pub two_factor: bool,
}

/// The platform economic config (`config` field of `GET /v1/config/economics`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicConfig {
    pub nominal_cents_per_credit: i64,
    pub credit_packs: Vec<CreditPack>,
    pub submission_fee_default: i64,
    pub creation_fees: CreationFees,
    pub rank: RankEconomics,
    #[serde(default)]
    pub submission_fee_by_tier: Option<TierAmounts>,
    #[serde(default)]
    pub min_bounty_by_tier: Option<TierAmounts>,
    #[serde(default)]
    pub featured_tags: Vec<String>,
    #[serde(default)]
    pub gates: Option<Gates>,
    #[serde(default)]
    pub signup_grant_credits: i64,
}

/// Response from `GET /v1/config/economics`: the active version + its config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicConfigResponse {
    pub version: i64,
    pub config: EconomicConfig,
}

/// A public user profile (`GET /v1/users/{handle}`) — public fields only.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicProfile {
    pub id: String,
    pub handle: String,
    pub display_name: Option<String>,
    /// Resolved seed for the deterministic generated avatar.
    pub avatar_seed: String,
    pub rank_tier: String,
    pub rank_level: i32,
    /// `admin` / `staff` for platform operators (else absent).
    #[serde(default)]
    pub staff: Option<String>,
    pub member_since: String,
    pub stats: PublicStats,
    pub leaderboard: PublicLeaderboard,
    pub badges: Vec<ProfileBadge>,
}

/// Headline public stats on a profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicStats {
    pub competitions_entered: i64,
    pub competitions_hosted: i64,
    pub earned_credits: i64,
    pub scored_count: i64,
    pub win_count: i64,
    pub top3_count: i64,
    pub points: f64,
}

/// A profile's leaderboard standing (lifetime).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicLeaderboard {
    pub ranked: bool,
    pub provisional: bool,
    pub global_rank: Option<i64>,
    pub global_total: i64,
    pub tier_rank: Option<i64>,
    pub tier_total: i64,
}

/// An earned badge shown on a profile (empty until the badge system ships).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBadge {
    pub slug: String,
    pub name: String,
    pub icon: String,
}

/// Result of gifting credits to another user (`POST /v1/credits/gift`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiftResponse {
    pub recipient_handle: String,
    pub amount: i64,
    /// The sender's balance after the gift.
    pub balance: i64,
}

/// A competition access row (`GET /v1/competitions/{id}/access`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRow {
    pub user_id: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    /// `requested` | `approved` | `invited` | `denied`.
    pub status: String,
    pub created_at: String,
}
