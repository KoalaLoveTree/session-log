use serde_json::json;

const WRITTEN: &str = "2020-01-01T00:00:00Z";
const EDITED: &str = "2020-01-02T00:00:00Z";
const LATER: &str = "2099-01-01T00:00:00Z";
const PURGED: &str = "2024-06-01T00:00:00Z";
const PURGED_AGAIN: &str = "2025-01-01T00:00:00Z";

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

fn phone_note(
    n: u32,
    body: &str,
    updated_at: &str,
    deleted_at: Option<&str>,
    synced_at: Option<&str>,
) -> serde_json::Value {
    json!({
        "id": uuid(n),
        "body": body,
        "created_at": WRITTEN,
        "updated_at": updated_at,
        "deleted_at": deleted_at,
        "synced_at": synced_at,
        "other_body": null,
    })
}

fn purge(n: u32, purged_at: &str) -> serde_json::Value {
    json!({ "id": uuid(n), "purged_at": purged_at })
}

fn sync_request(
    entries: serde_json::Value,
    purges: serde_json::Value,
) -> axum::http::Request<axum::body::Body> {
    let payload = json!({ "entries": entries, "purges": purges }).to_string();
    post_json("/api/sync", &payload)
}

async fn call(
    pool: &sqlx::PgPool,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> (axum::http::StatusCode, serde_json::Value) {
    send(api::app(pool.clone()), json_request(method, uri, body)).await
}

async fn sync(
    pool: &sqlx::PgPool,
    entries: serde_json::Value,
    purges: serde_json::Value,
) -> (axum::http::StatusCode, serde_json::Value) {
    send(api::app(pool.clone()), sync_request(entries, purges)).await
}

async fn get(pool: &sqlx::PgPool, path: &str) -> serde_json::Value {
    let (status, json) = call(pool, "GET", path, None).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    json
}

fn assert_note_fields(entry: &serde_json::Value) {
    let obj = entry.as_object().unwrap();
    assert_eq!(obj.len(), 5);
    for key in ["id", "body", "created_at", "updated_at", "deleted_at"] {
        assert!(obj.contains_key(key));
    }
}

fn assert_listed(entry: &serde_json::Value) {
    let obj = entry.as_object().unwrap();
    assert_eq!(obj.len(), 6);
    for key in [
        "id",
        "body",
        "created_at",
        "updated_at",
        "deleted_at",
        "other_body",
    ] {
        assert!(obj.contains_key(key));
    }
}

fn assert_stamped(entry: &serde_json::Value) {
    assert!(entry["synced_at"].is_string());
    assert_eq!(entry["updated_at"], entry["synced_at"]);
    assert!(entry["other_body"].is_null());
}

fn only_entry(synced: &serde_json::Value) -> &serde_json::Value {
    assert!(synced["purges"].as_array().unwrap().is_empty());
    assert_eq!(synced["entries"].as_array().unwrap().len(), 1);
    &synced["entries"][0]
}

async fn agree(pool: &sqlx::PgPool, n: u32, body: &str) -> serde_json::Value {
    let (status, synced) = sync(
        pool,
        json!([phone_note(n, body, EDITED, None, None)]),
        json!([]),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced).clone();
    assert_eq!(entry["id"], uuid(n));
    assert_eq!(entry["body"], body);
    assert_stamped(&entry);
    entry
}

async fn edit(pool: &sqlx::PgPool, n: u32, body: &str) {
    std::thread::sleep(std::time::Duration::from_millis(20));
    let (status, _) = call(
        pool,
        "PUT",
        &format!("/api/entries/{}", uuid(n)),
        Some(&format!(r#"{{"body":"{body}"}}"#)),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
}

#[sqlx::test]
async fn phone_only_note_is_stored(pool: sqlx::PgPool) {
    let (status, synced) = sync(
        &pool,
        json!([phone_note(1, "from phone", EDITED, None, None)]),
        json!([]),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["id"], uuid(1));
    assert_eq!(entry["body"], "from phone");
    assert_eq!(entry["created_at"], WRITTEN);
    assert!(entry["deleted_at"].is_null());
    assert_stamped(entry);

    let visible = get(&pool, "/api/entries").await;
    assert_listed(&visible["entries"][0]);
    assert_eq!(visible["entries"][0]["id"], uuid(1));
    assert_eq!(visible["entries"][0]["body"], "from phone");
    assert!(visible["entries"][0]["deleted_at"].is_null());
    assert!(visible["entries"][0]["other_body"].is_null());
}

#[sqlx::test]
async fn pc_only_note_is_sent_once(pool: sqlx::PgPool) {
    let (status, _) = call(
        &pool,
        "POST",
        "/api/entries",
        Some(&create_body(1, "from pc")),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);

    let (status, synced) = sync(&pool, json!([]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced).clone();
    assert_eq!(entry["id"], uuid(1));
    assert_eq!(entry["body"], "from pc");
    assert_stamped(&entry);

    let (status, again) = sync(&pool, json!([entry]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(again["entries"].as_array().unwrap().is_empty());
    assert!(again["purges"].as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn newer_phone_text_is_kept(pool: sqlx::PgPool) {
    let agreed = agree(&pool, 1, "old").await;
    let mut phone = agreed.clone();
    phone["body"] = json!("new");
    phone["updated_at"] = json!(LATER);

    let (status, synced) = sync(&pool, json!([phone]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["body"], "new");
    assert_stamped(entry);
    assert_ne!(entry["synced_at"], agreed["synced_at"]);

    let visible = get(&pool, "/api/entries").await;
    assert_eq!(visible["entries"][0]["body"], "new");
}

#[sqlx::test]
async fn newer_phone_soft_delete_is_kept(pool: sqlx::PgPool) {
    let mut phone = agree(&pool, 1, "gone").await;
    phone["updated_at"] = json!(LATER);
    phone["deleted_at"] = json!(LATER);

    let (status, synced) = sync(&pool, json!([phone]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["body"], "gone");
    assert_eq!(entry["deleted_at"], LATER);
    assert_stamped(entry);

    let visible = get(&pool, "/api/entries").await;
    assert!(visible["entries"].as_array().unwrap().is_empty());

    let deleted = get(&pool, "/api/entries/deleted").await;
    assert_eq!(deleted["entries"][0]["body"], "gone");
    assert_eq!(deleted["entries"][0]["deleted_at"], LATER);
}

#[sqlx::test]
async fn newer_pc_text_is_kept(pool: sqlx::PgPool) {
    let agreed = agree(&pool, 1, "old").await;
    edit(&pool, 1, "from pc").await;

    let (status, synced) = sync(&pool, json!([agreed]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["body"], "from pc");
    assert_stamped(entry);

    let visible = get(&pool, "/api/entries").await;
    assert_eq!(visible["entries"][0]["body"], "from pc");
}

#[sqlx::test]
async fn both_newer_keep_both_texts(pool: sqlx::PgPool) {
    let agreed = agree(&pool, 1, "shared").await;
    edit(&pool, 1, "pc text").await;
    let mut phone = agreed.clone();
    phone["body"] = json!("phone text");
    phone["updated_at"] = json!(LATER);

    let (status, synced) = sync(&pool, json!([phone]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["body"], "phone text");
    assert_eq!(entry["other_body"], "pc text");
    assert_eq!(entry["synced_at"], agreed["synced_at"]);

    let visible = get(&pool, "/api/entries").await;
    assert_listed(&visible["entries"][0]);
    assert_eq!(visible["entries"][0]["body"], "pc text");
    assert_eq!(visible["entries"][0]["other_body"], "phone text");
}

#[sqlx::test]
async fn both_newer_same_text_agrees(pool: sqlx::PgPool) {
    let agreed = agree(&pool, 1, "old").await;
    edit(&pool, 1, "same").await;
    let mut phone = agreed.clone();
    phone["body"] = json!("same");
    phone["updated_at"] = json!(LATER);

    let (status, synced) = sync(&pool, json!([phone]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["body"], "same");
    assert_stamped(entry);
    assert_ne!(entry["synced_at"], agreed["synced_at"]);
}

#[sqlx::test]
async fn unsynced_same_text_agrees(pool: sqlx::PgPool) {
    let (status, created) =
        call(&pool, "POST", "/api/entries", Some(&create_body(1, "same"))).await;
    assert_eq!(status, axum::http::StatusCode::CREATED);

    let (status, synced) = sync(
        &pool,
        json!([phone_note(1, "same", EDITED, None, None)]),
        json!([]),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["body"], "same");
    assert_eq!(entry["created_at"], WRITTEN);
    assert_stamped(entry);

    let visible = get(&pool, "/api/entries").await;
    assert_eq!(visible["entries"][0]["body"], "same");
    assert_eq!(visible["entries"][0]["created_at"], created["created_at"]);
}

#[sqlx::test]
async fn unsynced_different_texts_stay(pool: sqlx::PgPool) {
    let (status, created) = call(
        &pool,
        "POST",
        "/api/entries",
        Some(&create_body(1, "pc text")),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::CREATED);

    let (status, synced) = sync(
        &pool,
        json!([phone_note(1, "phone text", EDITED, None, None)]),
        json!([]),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    let entry = only_entry(&synced);
    assert_eq!(entry["body"], "phone text");
    assert_eq!(entry["other_body"], "pc text");
    assert!(entry["synced_at"].is_null());

    let visible = get(&pool, "/api/entries").await;
    assert_listed(&visible["entries"][0]);
    assert_eq!(visible["entries"][0]["body"], "pc text");
    assert_eq!(visible["entries"][0]["other_body"], "phone text");
    assert_eq!(visible["entries"][0]["created_at"], created["created_at"]);
}

#[sqlx::test]
async fn phone_purge_asks_then_accept_blocks_the_note(pool: sqlx::PgPool) {
    agree(&pool, 1, "note").await;
    let (status, synced) = sync(&pool, json!([]), json!([purge(1, PURGED)])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(synced["entries"].as_array().unwrap().is_empty());
    assert!(synced["purges"].as_array().unwrap().is_empty());

    let visible = get(&pool, "/api/entries").await;
    assert_eq!(visible["entries"][0]["body"], "note");

    let pending = get(&pool, "/api/purges/pending").await;
    assert_eq!(pending["purges"][0]["id"], uuid(1));
    assert_eq!(pending["purges"][0]["purged_at"], PURGED);
    assert_eq!(pending["purges"][0]["changed"], false);

    edit(&pool, 1, "edited").await;
    let pending = get(&pool, "/api/purges/pending").await;
    assert_eq!(pending["purges"][0]["changed"], true);

    let (status, _) = call(
        &pool,
        "POST",
        &format!("/api/purges/{}/accept", uuid(1)),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let visible = get(&pool, "/api/entries").await;
    assert!(visible["entries"].as_array().unwrap().is_empty());
    let deleted = get(&pool, "/api/entries/deleted").await;
    assert!(deleted["entries"].as_array().unwrap().is_empty());

    let (status, blocked) = sync(
        &pool,
        json!([phone_note(1, "resurrected", LATER, None, None)]),
        json!([]),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(blocked["entries"].as_array().unwrap().is_empty());
    assert_eq!(blocked["purges"][0]["id"], uuid(1));

    let visible = get(&pool, "/api/entries").await;
    assert!(visible["entries"].as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn pc_purge_asks_the_phone(pool: sqlx::PgPool) {
    let agreed = agree(&pool, 1, "note").await;
    let (status, _) = call(&pool, "DELETE", &format!("/api/entries/{}", uuid(1)), None).await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);
    let (status, _) = call(
        &pool,
        "DELETE",
        &format!("/api/entries/{}/purge", uuid(1)),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let (status, synced) = sync(&pool, json!([agreed]), json!([])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(synced["entries"].as_array().unwrap().is_empty());
    assert_eq!(synced["purges"][0]["id"], uuid(1));
    assert!(synced["purges"][0]["purged_at"].is_string());

    let visible = get(&pool, "/api/entries").await;
    assert!(visible["entries"].as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn decline_keeps_the_row_and_ignores_the_same_purge(pool: sqlx::PgPool) {
    let agreed = agree(&pool, 1, "kept").await;
    let (status, synced) = sync(&pool, json!([]), json!([purge(1, PURGED)])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(synced["entries"].as_array().unwrap().is_empty());

    let (status, entry) = call(
        &pool,
        "POST",
        &format!("/api/purges/{}/decline", uuid(1)),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_note_fields(&entry);
    assert_eq!(entry["body"], "kept");

    let (status, again) = sync(&pool, json!([agreed.clone()]), json!([purge(1, PURGED)])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(again["purges"].as_array().unwrap().is_empty());
    assert_eq!(again["entries"].as_array().unwrap().len(), 1);
    assert_eq!(again["entries"][0]["body"], "kept");

    let (status, later) = sync(&pool, json!([agreed]), json!([purge(1, PURGED_AGAIN)])).await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert!(later["entries"].as_array().unwrap().is_empty());
    assert_eq!(later["purges"][0]["id"], uuid(1));
    assert_eq!(later["purges"][0]["purged_at"], PURGED_AGAIN);

    let visible = get(&pool, "/api/entries").await;
    assert_eq!(visible["entries"][0]["body"], "kept");
}

#[sqlx::test]
async fn decline_inserts_the_supplied_copy(pool: sqlx::PgPool) {
    agree(&pool, 1, "gone").await;
    let (status, _) = call(&pool, "DELETE", &format!("/api/entries/{}", uuid(1)), None).await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);
    let (status, _) = call(
        &pool,
        "DELETE",
        &format!("/api/entries/{}/purge", uuid(1)),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

    let payload = phone_note(1, "back", EDITED, None, Some(EDITED)).to_string();
    let (status, entry) = call(
        &pool,
        "POST",
        &format!("/api/purges/{}/decline", uuid(1)),
        Some(&payload),
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK);
    assert_note_fields(&entry);
    assert_eq!(entry["body"], "back");
    assert_eq!(entry["created_at"], WRITTEN);
    assert_eq!(entry["updated_at"], EDITED);
    assert!(entry["deleted_at"].is_null());

    let visible = get(&pool, "/api/entries").await;
    assert_eq!(visible["entries"][0]["body"], "back");
    assert_eq!(visible["entries"][0]["created_at"], WRITTEN);
}

#[sqlx::test]
async fn sync_rejects_a_bad_note(pool: sqlx::PgPool) {
    let id = uuid(1);
    let cases = [
        (
            format!(
                r#"{{"entries":[{{"body":"hello","created_at":"{WRITTEN}","updated_at":"{EDITED}"}}],"purges":[]}}"#
            ),
            "id must be a uuid",
        ),
        (
            format!(
                r#"{{"entries":[{{"id":"7","body":"hello","created_at":"{WRITTEN}","updated_at":"{EDITED}"}}],"purges":[]}}"#
            ),
            "id must be a uuid",
        ),
        (
            format!(
                r#"{{"entries":[{{"id":"AAAAAAAA-BBBB-4CCC-8DDD-EEEEEEEEEEEE","body":"hello","created_at":"{WRITTEN}","updated_at":"{EDITED}"}}],"purges":[]}}"#
            ),
            "id must be a uuid",
        ),
        (
            format!(
                r#"{{"entries":[{{"id":"{id}","body":"   ","created_at":"{WRITTEN}","updated_at":"{EDITED}"}}],"purges":[]}}"#
            ),
            "body must not be empty",
        ),
        (
            format!(
                r#"{{"entries":[{{"id":"{id}","body":"hello","created_at":"yesterday","updated_at":"{EDITED}"}}],"purges":[]}}"#
            ),
            "bad time",
        ),
    ];
    for (body, error) in cases {
        let (status, json) = call(&pool, "POST", "/api/sync", Some(&body)).await;
        assert_eq!(status, axum::http::StatusCode::BAD_REQUEST);
        assert_eq!(json["error"], error);
    }

    let visible = get(&pool, "/api/entries").await;
    assert!(visible["entries"].as_array().unwrap().is_empty());
}

#[sqlx::test]
async fn missing_purge_is_not_found(pool: sqlx::PgPool) {
    let (status, json) = call(
        &pool,
        "POST",
        &format!("/api/purges/{}/accept", uuid(1)),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(json["error"], "not found");

    let (status, json) = call(
        &pool,
        "POST",
        &format!("/api/purges/{}/decline", uuid(1)),
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::NOT_FOUND);
    assert_eq!(json["error"], "not found");
}
