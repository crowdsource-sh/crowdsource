//! Integration tests for the `Client` against a mocked server (wiremock).

use crowdsource::{
    BountyMode, Client, CompetitionMetric, CompetitionQuery, CompetitionType, CreateCompetition,
    CrowdsourceError,
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
        bounty_weights: None,
        dataset_schema: None,
        oracle_url: None,
        oracle_auth_header: None,
        resolution_offset_minutes: None,
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
