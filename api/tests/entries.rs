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

fn uuid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn create_body(n: u32, body: &str) -> String {
    format!(r#"{{"id":"{}","body":"{body}"}}"#, uuid(n))
}

#[sqlx::test]
async fn empty_body_is_rejected(pool: sqlx::PgPool) {
    let (status, json) = send(
        api::app(pool),
        post_json("/api/entries", &create_body(1, "   ")),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
    assert_eq!(json["error"], "body must not be empty");
}

#[sqlx::test]
async fn id_must_be_a_uuid(pool: sqlx::PgPool) {
    for body in [
        r#"{"body":"hello"}"#,
        r#"{"id":"7","body":"hello"}"#,
        r#"{"id":"AAAAAAAA-BBBB-4CCC-8DDD-EEEEEEEEEEEE","body":"hello"}"#,
    ] {
        let (status, json) = send(api::app(pool.clone()), post_json("/api/entries", body)).await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], "id must be a uuid");
    }
}

#[sqlx::test]
async fn duplicate_id_is_rejected(pool: sqlx::PgPool) {
    let body = create_body(1, "hello");
    let (status, _) = send(api::app(pool.clone()), post_json("/api/entries", &body)).await;
    assert_eq!(status, axum::http::StatusCode::CREATED);

    let (status, json) = send(api::app(pool), post_json("/api/entries", &body)).await;
    assert_eq!(status, axum::http::StatusCode::CONFLICT);
    assert_eq!(json["error"], "id already used");
}

#[sqlx::test]
async fn create_then_list(pool: sqlx::PgPool) {
    let (status, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", &create_body(1, "hello")),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);
    assert_eq!(created["id"], uuid(1));
    assert_eq!(created["body"], "hello");
    assert!(created["deleted_at"].is_null());

    let (status, list) = send(api::app(pool), json_request("GET", "/api/entries", None)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(list["entries"][0]["id"], uuid(1));
    assert_eq!(list["entries"][0]["body"], "hello");
}

#[sqlx::test]
async fn legacy_text_id_still_edits(pool: sqlx::PgPool) {
    sqlx::query(
        "INSERT INTO entries (id, body, created_at, updated_at)
         VALUES ('7', 'old', now(), now())",
    )
    .execute(&pool)
    .await
    .unwrap();

    let (_, before) = send(
        api::app(pool.clone()),
        json_request("GET", "/api/entries", None),
    )
    .await;
    let created_at = before["entries"][0]["created_at"].clone();

    let (status, updated) = send(
        api::app(pool.clone()),
        json_request("PUT", "/api/entries/7", Some(r#"{"body":"new"}"#)),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(updated["id"], "7");
    assert_eq!(updated["body"], "new");
    assert_eq!(updated["created_at"], created_at);

    let (_, list) = send(api::app(pool), json_request("GET", "/api/entries", None)).await;
    assert_eq!(list["entries"][0]["id"], "7");
    assert_eq!(list["entries"][0]["body"], "new");
}

#[sqlx::test]
async fn updated_at_moves_on_edit_delete_and_restore(pool: sqlx::PgPool) {
    let (_, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", &create_body(1, "old")),
    )
    .await;
    let id = created["id"].as_str().unwrap();
    let created_at = created["created_at"].as_str().unwrap().to_string();
    let updated_at = created["updated_at"].as_str().unwrap().to_string();

    std::thread::sleep(std::time::Duration::from_millis(20));
    let (status, edited) = send(
        api::app(pool.clone()),
        json_request(
            "PUT",
            &format!("/api/entries/{id}"),
            Some(r#"{"body":"new"}"#),
        ),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(edited["created_at"], created_at);
    assert!(edited["updated_at"].as_str().unwrap() > updated_at.as_str());

    std::thread::sleep(std::time::Duration::from_millis(20));
    let (status, _) = send(
        api::app(pool.clone()),
        json_request("DELETE", &format!("/api/entries/{id}"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (_, deleted) = send(
        api::app(pool.clone()),
        json_request("GET", "/api/entries/deleted", None),
    )
    .await;
    let row = &deleted["entries"][0];
    assert!(row["deleted_at"].is_string());
    assert!(row["updated_at"].as_str().unwrap() > edited["updated_at"].as_str().unwrap());

    std::thread::sleep(std::time::Duration::from_millis(20));
    let (status, restored) = send(
        api::app(pool),
        json_request("POST", &format!("/api/entries/{id}/restore"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(restored["deleted_at"].is_null());
    assert_eq!(restored["created_at"], created_at);
    assert!(restored["updated_at"].as_str().unwrap() > row["updated_at"].as_str().unwrap());
}

#[sqlx::test]
async fn visible_list_caps_at_50(pool: sqlx::PgPool) {
    for n in 1..=51 {
        let (status, _) = send(
            api::app(pool.clone()),
            post_json("/api/entries", &create_body(n, "note")),
        )
        .await;
        assert_eq!(status, axum::http::StatusCode::CREATED);
    }

    let (status, list) = send(api::app(pool), json_request("GET", "/api/entries", None)).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_eq!(list["entries"].as_array().unwrap().len(), 50);
}

#[sqlx::test]
async fn update_entry(pool: sqlx::PgPool) {
    let (_, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", &create_body(1, "old")),
    )
    .await;
    let id = created["id"].as_str().unwrap();
    let created_at = created["created_at"].clone();

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
    assert_eq!(updated["created_at"], created_at);
}

#[sqlx::test]
async fn delete_restore_purge(pool: sqlx::PgPool) {
    let (_, created) = send(
        api::app(pool.clone()),
        post_json("/api/entries", &create_body(1, "temp")),
    )
    .await;
    let id = created["id"].as_str().unwrap();

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
        post_json("/api/entries", &create_body(1, "keep")),
    )
    .await;
    let id = created["id"].as_str().unwrap();

    let (status, json) = send(
        api::app(pool),
        json_request("DELETE", &format!("/api/entries/{id}/purge"), None),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(json["error"], "not found");
}
