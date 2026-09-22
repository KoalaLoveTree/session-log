#[derive(Clone)]
struct AppState {
    pool: sqlx::PgPool,
}

#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
struct Entry {
    id: String,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct EntryList {
    entries: Vec<Entry>,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct NewEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    body: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
struct EntryUpdate {
    #[serde(default)]
    body: String,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct ErrorBody {
    error: &'static str,
}

enum ApiError {
    EmptyBody,
    BadId,
    DuplicateId,
    NotFound,
    Db(sqlx::Error),
}

fn is_uuid(id: &str) -> bool {
    let mut parts = id.split('-');
    for width in [8, 4, 4, 4, 12] {
        let Some(part) = parts.next() else {
            return false;
        };
        if part.len() != width {
            return false;
        }
        if !part
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return false;
        }
    }
    parts.next().is_none()
}

fn trimmed_body(body: &str) -> Result<&str, ApiError> {
    let body = body.trim();
    if body.is_empty() {
        Err(ApiError::EmptyBody)
    } else {
        Ok(body)
    }
}

fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .and_then(|db| db.code())
        .is_some_and(|code| code == "23505")
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::EmptyBody => (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(ErrorBody {
                    error: "body must not be empty",
                }),
            )
                .into_response(),
            ApiError::BadId => (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(ErrorBody {
                    error: "id must be a uuid",
                }),
            )
                .into_response(),
            ApiError::DuplicateId => (
                axum::http::StatusCode::CONFLICT,
                axum::Json(ErrorBody {
                    error: "id already used",
                }),
            )
                .into_response(),
            ApiError::NotFound => (
                axum::http::StatusCode::NOT_FOUND,
                axum::Json(ErrorBody { error: "not found" }),
            )
                .into_response(),
            ApiError::Db(err) => {
                eprintln!("db: {err}");
                (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(ErrorBody { error: "internal" }),
                )
                    .into_response()
            }
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/entries",
    request_body = NewEntry,
    responses(
        (status = 201, description = "Created", body = Entry),
        (status = 400, description = "Bad id or empty body", body = ErrorBody),
        (status = 409, description = "Id already used", body = ErrorBody)
    )
)]
async fn create_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    payload: Result<axum::Json<NewEntry>, axum::extract::rejection::JsonRejection>,
) -> Result<(axum::http::StatusCode, axum::Json<Entry>), ApiError> {
    let axum::Json(new) = payload.map_err(|_| ApiError::EmptyBody)?;
    let id = new.id.trim();
    if !is_uuid(id) {
        return Err(ApiError::BadId);
    }
    let body = trimmed_body(&new.body)?;

    let entry = sqlx::query_as::<_, Entry>(
        "INSERT INTO entries (id, body, created_at, updated_at)
         VALUES ($1, $2, now(), now())
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(id)
    .bind(body)
    .fetch_one(&state.pool)
    .await
    .map_err(|err| {
        if is_unique_violation(&err) {
            ApiError::DuplicateId
        } else {
            ApiError::Db(err)
        }
    })?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(entry)))
}

#[utoipa::path(
    get,
    path = "/api/entries",
    responses((status = 200, description = "Visible entries", body = EntryList))
)]
async fn list_entries(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<EntryList>, ApiError> {
    let entries = sqlx::query_as::<_, Entry>(
        "SELECT id, body, created_at, updated_at, deleted_at
         FROM entries
         WHERE deleted_at IS NULL
         ORDER BY created_at DESC, id DESC
         LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    Ok(axum::Json(EntryList { entries }))
}

#[utoipa::path(
    get,
    path = "/api/entries/deleted",
    responses((status = 200, description = "Soft-deleted entries", body = EntryList))
)]
async fn list_deleted(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<EntryList>, ApiError> {
    let entries = sqlx::query_as::<_, Entry>(
        "SELECT id, body, created_at, updated_at, deleted_at
         FROM entries
         WHERE deleted_at IS NOT NULL
         ORDER BY deleted_at DESC, id DESC
         LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    Ok(axum::Json(EntryList { entries }))
}

#[utoipa::path(
    put,
    path = "/api/entries/{id}",
    params(("id" = String, Path, description = "Entry id")),
    request_body = EntryUpdate,
    responses(
        (status = 200, description = "Updated", body = Entry),
        (status = 400, description = "Empty body", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
async fn update_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    payload: Result<axum::Json<EntryUpdate>, axum::extract::rejection::JsonRejection>,
) -> Result<axum::Json<Entry>, ApiError> {
    let axum::Json(new) = payload.map_err(|_| ApiError::EmptyBody)?;
    let body = trimmed_body(&new.body)?;

    let entry = sqlx::query_as::<_, Entry>(
        "UPDATE entries
         SET body = $1, updated_at = now()
         WHERE id = $2 AND deleted_at IS NULL
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(body)
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    match entry {
        Some(entry) => Ok(axum::Json(entry)),
        None => Err(ApiError::NotFound),
    }
}

#[utoipa::path(
    delete,
    path = "/api/entries/{id}",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 204, description = "Soft-deleted"),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
async fn delete_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let result = sqlx::query(
        "UPDATE entries
         SET deleted_at = now(), updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/entries/{id}/restore",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 200, description = "Restored", body = Entry),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
async fn restore_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::Json<Entry>, ApiError> {
    let entry = sqlx::query_as::<_, Entry>(
        "UPDATE entries
         SET deleted_at = NULL, updated_at = now()
         WHERE id = $1 AND deleted_at IS NOT NULL
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    match entry {
        Some(entry) => Ok(axum::Json(entry)),
        None => Err(ApiError::NotFound),
    }
}

#[utoipa::path(
    delete,
    path = "/api/entries/{id}/purge",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 204, description = "Purged"),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
async fn purge_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let result = sqlx::query("DELETE FROM entries WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(id)
        .execute(&state.pool)
        .await
        .map_err(ApiError::Db)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        list_entries,
        list_deleted,
        create_entry,
        update_entry,
        delete_entry,
        restore_entry,
        purge_entry
    ),
    components(schemas(Entry, EntryList, NewEntry, EntryUpdate, ErrorBody))
)]
struct ApiDoc;

async fn swagger_dark_css() -> impl axum::response::IntoResponse {
    (
        [(axum::http::header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../swagger-dark.css"),
    )
}

async fn inject_swagger_dark(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let response = next.run(request).await;
    let content_type = response
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    if !content_type.contains("text/html") {
        return response;
    }
    let (mut parts, body) = response.into_parts();
    let Ok(bytes) = axum::body::to_bytes(body, 1_000_000).await else {
        return axum::response::Response::from_parts(parts, axum::body::Body::empty());
    };
    let html = String::from_utf8_lossy(&bytes);
    let html = if html.contains("dark.css") {
        html.into_owned()
    } else {
        html.replace(
            "</head>",
            r#"<link rel="stylesheet" href="/api/docs/dark.css" /></head>"#,
        )
    };
    parts.headers.remove(axum::http::header::CONTENT_LENGTH);
    axum::response::Response::from_parts(parts, axum::body::Body::from(html))
}

pub fn app(pool: sqlx::PgPool) -> axum::Router {
    axum::Router::new()
        .route("/api/docs/dark.css", axum::routing::get(swagger_dark_css))
        .merge(
            utoipa_swagger_ui::SwaggerUi::new("/api/docs")
                .url("/api/openapi.json", <ApiDoc as utoipa::OpenApi>::openapi())
                .config(
                    utoipa_swagger_ui::Config::from("/api/openapi.json").with_syntax_highlight(
                        utoipa_swagger_ui::SyntaxHighlight::default().theme("monokai"),
                    ),
                ),
        )
        .layer(axum::middleware::from_fn(inject_swagger_dark))
        .route(
            "/api/entries",
            axum::routing::get(list_entries).post(create_entry),
        )
        .route("/api/entries/deleted", axum::routing::get(list_deleted))
        .route(
            "/api/entries/{id}",
            axum::routing::put(update_entry).delete(delete_entry),
        )
        .route(
            "/api/entries/{id}/restore",
            axum::routing::post(restore_entry),
        )
        .route(
            "/api/entries/{id}/purge",
            axum::routing::delete(purge_entry),
        )
        .with_state(AppState { pool })
}
