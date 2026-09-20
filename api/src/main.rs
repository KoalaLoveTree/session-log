#[derive(Clone)]
struct AppState {
    pool: sqlx::PgPool,
}

#[derive(serde::Serialize, sqlx::FromRow)]
struct Entry {
    id: i64,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
struct EntryList {
    entries: Vec<Entry>,
}

#[derive(serde::Deserialize)]
struct NewEntry {
    #[serde(default)]
    body: String,
}

#[derive(serde::Serialize)]
struct ErrorBody {
    error: &'static str,
}

enum ApiError {
    EmptyBody,
    NotFound,
    Db(sqlx::Error),
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

async fn create_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    payload: Result<axum::Json<NewEntry>, axum::extract::rejection::JsonRejection>,
) -> Result<(axum::http::StatusCode, axum::Json<Entry>), ApiError> {
    let axum::Json(new) = payload.map_err(|_| ApiError::EmptyBody)?;
    let body = new.body.trim();
    if body.is_empty() {
        return Err(ApiError::EmptyBody);
    }

    let entry = sqlx::query_as::<_, Entry>(
        "INSERT INTO entries (body) VALUES ($1) RETURNING id, body, created_at",
    )
    .bind(body)
    .fetch_one(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(entry)))
}

async fn list_entries(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<EntryList>, ApiError> {
    let entries = sqlx::query_as::<_, Entry>(
        "SELECT id, body, created_at FROM entries WHERE deleted_at IS NULL ORDER BY created_at DESC, id DESC LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    Ok(axum::Json(EntryList { entries }))
}

async fn delete_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<axum::http::StatusCode, ApiError> {
    let result =
        sqlx::query("UPDATE entries SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL")
            .bind(id)
            .execute(&state.pool)
            .await
            .map_err(ApiError::Db)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = axum::Router::new()
        .route(
            "/api/entries",
            axum::routing::get(list_entries).post(create_entry),
        )
        .route("/api/entries/{id}", axum::routing::delete(delete_entry))
        .with_state(AppState { pool });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
