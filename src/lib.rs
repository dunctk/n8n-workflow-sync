pub mod api;
pub mod config;

#[cfg(test)]
mod tests {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use url::Url;

    #[tokio::test]
    async fn list_workflows() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/workflows"))
            .and(header("X-N8N-API-KEY", "test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": [{ "id": "1", "name": "Test" }]
            })))
            .mount(&server)
            .await;

        let config = crate::config::N8nConfig {
            api_key: "test-key".into(),
            host: Url::parse(&server.uri()).unwrap(),
        };
        let workflows = crate::api::list_workflows(&config).await.unwrap();
        // ensure the mock was hit
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
        assert_eq!(workflows.len(), 1);
        assert_eq!(workflows[0].id, "1");
        assert_eq!(workflows[0].name, "Test");
    }

    #[tokio::test]
    async fn create_workflow() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/workflows"))
            .and(header("X-N8N-API-KEY", "test-key"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "id": "2",
                    "name": "New"
                })),
            )
            .mount(&server)
            .await;

        let cfg = crate::config::N8nConfig {
            api_key: "test-key".into(),
            host: Url::parse(&server.uri()).unwrap(),
        };
        let wf = crate::api::create_workflow(&cfg, "New").await.unwrap();
        assert_eq!(wf.id, "2");
        assert_eq!(wf.name, "New");
    }
}
