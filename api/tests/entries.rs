async fn send(
    app: axum::Router,
    request: axum::http::Request<axum::body::Body>,
) -> (axum::http::StatusCode, serde_json::Value) {
    let response = tower::ServiceExt::oneshot(app, request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
        .await
        .unwrap();
    let json = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    };
    (status, json)
}

fn post_json(uri: &str, body: &str) -> axum::http::Request<axum::body::Body> {
    axum::http::Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .unwrap()
}

fn json_request(
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> axum::http::Request<axum::body::Body> {
    let mut builder = axum::http::Request::builder().method(method).uri(uri);
    let payload = if let Some(body) = body {
        builder = builder.header("content-type", "application/json");
        axum::body::Body::from(body.to_string())
    } else {
        axum::body::Body::empty()
    };
    builder.body(payload).unwrap()
}

#[sqlx::test]
async fn empty_body_is_rejected(pool: sqlx::PgPool) {
    let (status, json) = send(
        api::app(pool),
        post_json("/api/entries", r#"{"body":"   "}"#),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(json["error"], "body must not be empty");
}

#[sqlx::test]
async fn create_then_list(pool: sqlx::PgPool) {
    let (status, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", r#"{"body":"hello"}"#),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(created["body"], "hello");

    let (status, list) = send(api::app(pool), json_request("GET", "/api/entries", None)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(list["entries"][0]["body"], "hello");
}

#[sqlx::test]
async fn update_entry(pool: sqlx::PgPool) {
    let (_, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", r#"{"body":"old"}"#),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, updated) = send(
        api::app(pool.clone()),
        json_request(
            "PUT",
            &format!("/api/entries/{id}"),
            Some(r#"{"body":"new"}"#),
        ),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(updated["body"], "new");
}

#[sqlx::test]
async fn delete_restore_purge(pool: sqlx::PgPool) {
    let (_, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", r#"{"body":"temp"}"#),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, _) = send(
        api::app(pool.clone()),
        json_request("DELETE", &format!("/api/entries/{id}"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (_, visible) = send(
        api::app(pool.clone()),
        json_request("GET", "/api/entries", None),
    )
    .await;
    assert_eq!(visible["entries"].as_array().unwrap().len(), 0);

    let (_, deleted) = send(
        api::app(pool.clone()),
        json_request("GET", "/api/entries/deleted", None),
    )
    .await;
    assert_eq!(deleted["entries"][0]["id"], id);

    let (status, _) = send(
        api::app(pool.clone()),
        json_request("POST", &format!("/api/entries/{id}/restore"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);

    let (status, _) = send(
        api::app(pool.clone()),
        json_request("DELETE", &format!("/api/entries/{id}"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (status, _) = send(
        api::app(pool.clone()),
        json_request("DELETE", &format!("/api/entries/{id}/purge"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (_, deleted) = send(
        api::app(pool),
        json_request("GET", "/api/entries/deleted", None),
    )
    .await;
    assert_eq!(deleted["entries"].as_array().unwrap().len(), 0);
}

#[sqlx::test]
async fn purge_visible_is_not_found(pool: sqlx::PgPool) {
    let (_, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", r#"{"body":"keep"}"#),
    )
    .await;
    let id = created["id"].as_i64().unwrap();

    let (status, json) = send(
        api::app(pool),
        json_request("DELETE", &format!("/api/entries/{id}/purge"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(json["error"], "not found");
}
