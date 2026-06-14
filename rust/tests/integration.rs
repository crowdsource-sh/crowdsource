//! Integration tests for the `Client` against a mocked server (wiremock).

use crowdsource::{
    BountyMode, Client, CompetitionMetric, CompetitionQuery, CompetitionType, CreateCompetition,
    CreateSubmission, CrowdsourceError,
};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn fixed_end() -> chrono::DateTime<chrono::Utc> {
    "2026-06-06T17:00:00Z".parse().unwrap()
}

#[tokio::test]
async fn lists_competitions_sends_api_key() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/competitions"))
        .and(header("x-api-key", "cs_sk_test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "competitions": [],
            "total": 0,
            "limit": 20,
            "offset": 0
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri(), Some("cs_sk_test".to_string())).unwrap();
    let res = client
        .list_competitions(&CompetitionQuery::default())
        .await
        .unwrap();
    assert_eq!(res.total, 0);
}

#[tokio::test]
async fn create_competition_surfaces_rfc7807_detail() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/competitions"))
        .respond_with(ResponseTemplate::new(402).set_body_json(serde_json::json!({
            "type": "https://httpstatuses.io/402",
            "title": "Payment Required",
            "status": 402,
            "detail": "insufficient credits: need 1100, have 50"
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri(), None).unwrap();
    let req = CreateCompetition {
        title: "t".to_string(),
        description: "predict the thing".to_string(),
        competition_type: CompetitionType::Classification,
        metric: CompetitionMetric::Accuracy,
        bounty_amount: 1000,
        bounty_mode: BountyMode::TopNEqual,
        bounty_top_n: 3,
        end_date: fixed_end(),
        ..Default::default()
    };
    let err = client.create_competition(&req).await.unwrap_err();
    match err {
        CrowdsourceError::Api(p) => {
            assert_eq!(p.status, 402);
            assert!(p.to_string().contains("insufficient credits"));
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[tokio::test]
async fn submit_inline_payload() {
    let server = MockServer::start().await;
    let cid = "00000000-0000-0000-0000-000000000010";
    Mock::given(method("POST"))
        .and(path(format!("/v1/competitions/{cid}/submissions")))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000020",
            "competition_id": cid,
            "user_id": "00000000-0000-0000-0000-000000000030",
            "s3_key": "",
            "score": null,
            "rank": null,
            "payout": null,
            "created_at": "2026-06-06T09:00:00Z",
            "scored_at": null
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri(), Some("cs_sk_test".to_string())).unwrap();
    let body = CreateSubmission::from_payload(serde_json::json!({"value": 42}));
    let sub = client.submit(cid.parse().unwrap(), &body).await.unwrap();
    assert_eq!(sub.competition_id.to_string(), cid);
    assert!(sub.score.is_none());
}

#[tokio::test]
async fn create_api_key_returns_secret_once() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/api-keys"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "00000000-0000-0000-0000-000000000040",
            "name": "ci",
            "secret": "cs_deadbeef",
            "created_at": "2026-06-06T09:00:00Z"
        })))
        .mount(&server)
        .await;

    let client = Client::new(server.uri(), Some("cs_sk_test".to_string())).unwrap();
    let key = client.create_api_key("ci").await.unwrap();
    assert_eq!(key.secret, "cs_deadbeef");
}

#[tokio::test]
async fn get_retries_on_5xx_then_succeeds() {
    let server = MockServer::start().await;
    // First response 503, then 200. wiremock serves mounts in order with
    // `up_to_n_times`, so mount the failure first (once) then the success.
    Mock::given(method("GET"))
        .and(path("/v1/summary"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .with_priority(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/v1/summary"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "competitions_open": 1,
            "competitions_total": 2,
            "active_bounties": 100,
            "predictions_today": 3,
            "predictions_total": 9,
            "predictors": 4,
            "credits_paid": 500
        })))
        .with_priority(2)
        .mount(&server)
        .await;

    let client = Client::new(server.uri(), None).unwrap();
    let s = client.summary().await.unwrap();
    assert_eq!(s.competitions_open, 1);
    assert_eq!(s.credits_paid, 500);
}
