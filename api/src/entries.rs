#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub(crate) struct Entry {
    id: String,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub(crate) struct ListedEntry {
    id: String,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    other_body: Option<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct EntryList {
    entries: Vec<ListedEntry>,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct NewEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    body: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(crate) struct EntryUpdate {
    #[serde(default)]
    body: String,
}

pub(crate) fn is_uuid(id: &str) -> bool {
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

pub(crate) fn trimmed_body(body: &str) -> Result<&str, crate::error::ApiError> {
    let body = body.trim();
    if body.is_empty() {
        Err(crate::error::ApiError::EmptyBody)
    } else {
        Ok(body)
    }
}

#[utoipa::path(
    post,
    path = "/api/entries",
    request_body = NewEntry,
    responses(
        (status = 201, description = "Created", body = Entry),
        (status = 400, description = "Bad id or empty body", body = crate::error::ErrorBody),
        (status = 409, description = "Id already used", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn create_entry(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    payload: Result<axum::Json<NewEntry>, axum::extract::rejection::JsonRejection>,
) -> Result<(axum::http::StatusCode, axum::Json<Entry>), crate::error::ApiError> {
    let axum::Json(new) = payload.map_err(|_| crate::error::ApiError::EmptyBody)?;
    let id = new.id.trim();
    if !is_uuid(id) {
        return Err(crate::error::ApiError::BadId);
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
        if crate::error::is_unique_violation(&err) {
            crate::error::ApiError::DuplicateId
        } else {
            crate::error::ApiError::Db(err)
        }
    })?;

    Ok((axum::http::StatusCode::CREATED, axum::Json(entry)))
}

#[utoipa::path(
    get,
    path = "/api/entries",
    responses((status = 200, description = "Visible entries, each with other_body", body = EntryList))
)]
pub(crate) async fn list_entries(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<axum::Json<EntryList>, crate::error::ApiError> {
    let entries = sqlx::query_as::<_, ListedEntry>(
        "SELECT id, body, created_at, updated_at, deleted_at, other_body
         FROM entries
         WHERE deleted_at IS NULL
         ORDER BY created_at DESC, id DESC
         LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(crate::error::ApiError::Db)?;

    Ok(axum::Json(EntryList { entries }))
}

#[utoipa::path(
    get,
    path = "/api/entries/deleted",
    responses((status = 200, description = "Soft-deleted entries, each with other_body", body = EntryList))
)]
pub(crate) async fn list_deleted(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<axum::Json<EntryList>, crate::error::ApiError> {
    let entries = sqlx::query_as::<_, ListedEntry>(
        "SELECT id, body, created_at, updated_at, deleted_at, other_body
         FROM entries
         WHERE deleted_at IS NOT NULL
         ORDER BY deleted_at DESC, id DESC
         LIMIT 50",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(crate::error::ApiError::Db)?;

    Ok(axum::Json(EntryList { entries }))
}

#[utoipa::path(
    put,
    path = "/api/entries/{id}",
    params(("id" = String, Path, description = "Entry id")),
    request_body = EntryUpdate,
    responses(
        (status = 200, description = "Updated. When other_body is set, that field and deleted_at are cleared", body = Entry),
        (status = 400, description = "Empty body", body = crate::error::ErrorBody),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn update_entry(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    payload: Result<axum::Json<EntryUpdate>, axum::extract::rejection::JsonRejection>,
) -> Result<axum::Json<Entry>, crate::error::ApiError> {
    let axum::Json(new) = payload.map_err(|_| crate::error::ApiError::EmptyBody)?;
    let body = trimmed_body(&new.body)?;

    let entry = sqlx::query_as::<_, Entry>(
        "UPDATE entries
         SET body = $1,
             updated_at = now(),
             other_body = CASE WHEN other_body IS NOT NULL THEN NULL ELSE other_body END,
             deleted_at = CASE WHEN other_body IS NOT NULL THEN NULL ELSE deleted_at END
         WHERE id = $2
           AND (other_body IS NOT NULL OR deleted_at IS NULL)
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(body)
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::error::ApiError::Db)?;

    match entry {
        Some(entry) => Ok(axum::Json(entry)),
        None => Err(crate::error::ApiError::NotFound),
    }
}

#[utoipa::path(
    delete,
    path = "/api/entries/{id}",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 204, description = "Soft-deleted"),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn delete_entry(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, crate::error::ApiError> {
    let result = sqlx::query(
        "UPDATE entries
         SET deleted_at = now(), updated_at = now()
         WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(id)
    .execute(&state.pool)
    .await
    .map_err(crate::error::ApiError::Db)?;

    if result.rows_affected() == 0 {
        return Err(crate::error::ApiError::NotFound);
    }
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/entries/{id}/restore",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 200, description = "Restored", body = Entry),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn restore_entry(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::Json<Entry>, crate::error::ApiError> {
    let entry = sqlx::query_as::<_, Entry>(
        "UPDATE entries
         SET deleted_at = NULL, updated_at = now()
         WHERE id = $1 AND deleted_at IS NOT NULL
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await
    .map_err(crate::error::ApiError::Db)?;

    match entry {
        Some(entry) => Ok(axum::Json(entry)),
        None => Err(crate::error::ApiError::NotFound),
    }
}

#[utoipa::path(
    delete,
    path = "/api/entries/{id}/purge",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 204, description = "Purged. A note that had synced is recorded in purges"),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn purge_entry(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, crate::error::ApiError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(crate::error::ApiError::Db)?;
    let synced_at = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT synced_at FROM entries WHERE id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(&id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(crate::error::ApiError::Db)?;
    let Some(synced_at) = synced_at else {
        return Err(crate::error::ApiError::NotFound);
    };
    if synced_at.is_some() {
        sqlx::query(
            "INSERT INTO purges (id, purged_at)
             VALUES ($1, now())
             ON CONFLICT DO NOTHING",
        )
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(crate::error::ApiError::Db)?;
    }
    sqlx::query("DELETE FROM entries WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(crate::error::ApiError::Db)?;
    tx.commit().await.map_err(crate::error::ApiError::Db)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
