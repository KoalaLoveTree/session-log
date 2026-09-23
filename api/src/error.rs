#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct ErrorBody {
    error: &'static str,
}

pub(crate) enum ApiError {
    EmptyBody,
    BadId,
    DuplicateId,
    NotFound,
    BadTime,
    Db(sqlx::Error),
}

pub(crate) fn is_unique_violation(err: &sqlx::Error) -> bool {
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
            ApiError::BadTime => (
                axum::http::StatusCode::BAD_REQUEST,
                axum::Json(ErrorBody { error: "bad time" }),
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
