use axum::extract::rejection::JsonRejection;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;
use sqlx::{FromRow, PgPool};
use std::env;

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[derive(Serialize, FromRow)]
struct Entry {
    id: i64,
    body: String,
    created_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct EntryList {
    entries: Vec<Entry>,
}

#[derive(Deserialize)]
struct NewEntry {
    #[serde(default)]
    body: String,
}

#[derive(Serialize)]
struct ErrorBody {
    error: &'static str,
}

enum ApiError {
    EmptyBody,
    Db(sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::EmptyBody => (
                StatusCode::BAD_REQUEST,
                Json(ErrorBody {
                    error: "body must not be empty",
                }),
            )
                .into_response(),
            ApiError::Db(err) => {
                eprintln!("db: {err}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorBody { error: "internal" }),
                )
                    .into_response()
            }
        }
    }
}

async fn create_entry(
    State(state): State<AppState>,
    payload: Result<Json<NewEntry>, JsonRejection>,
) -> Result<(StatusCode, Json<Entry>), ApiError> {
    let Json(new) = payload.map_err(|_| ApiError::EmptyBody)?;
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

    Ok((StatusCode::CREATED, Json(entry)))
}

async fn list_entries(State(state): State<AppState>) -> Result<Json<EntryList>, ApiError> {
    let entries = sqlx::query_as::<_, Entry>(
        "SELECT id, body, created_at FROM entries ORDER BY created_at DESC, id DESC LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Db)?;

    Ok(Json(EntryList { entries }))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = env::var("DATABASE_URL")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    sqlx::migrate!("./migrations").run(&pool).await?;

    let app = Router::new()
        .route("/api/entries", get(list_entries).post(create_entry))
        .with_state(AppState { pool });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
