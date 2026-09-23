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
    BadTime,
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
        (status = 204, description = "Purged. A note that had synced is recorded in purges"),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
async fn purge_entry(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let mut tx = state.pool.begin().await.map_err(ApiError::Db)?;
    let synced_at = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT synced_at FROM entries WHERE id = $1 AND deleted_at IS NOT NULL",
    )
    .bind(&id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Db)?;
    let Some(synced_at) = synced_at else {
        return Err(ApiError::NotFound);
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
        .map_err(ApiError::Db)?;
    }
    sqlx::query("DELETE FROM entries WHERE id = $1 AND deleted_at IS NOT NULL")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Db)?;
    tx.commit().await.map_err(ApiError::Db)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct SyncEntry {
    id: String,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    synced_at: Option<chrono::DateTime<chrono::Utc>>,
    other_body: Option<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema, sqlx::FromRow)]
struct SyncPurge {
    id: String,
    purged_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct SyncPayload {
    entries: Vec<SyncEntry>,
    purges: Vec<SyncPurge>,
}

#[derive(serde::Serialize, utoipa::ToSchema, sqlx::FromRow)]
struct PendingPurge {
    id: String,
    purged_at: chrono::DateTime<chrono::Utc>,
    changed: bool,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
struct PendingPurgeList {
    purges: Vec<PendingPurge>,
}

#[derive(sqlx::FromRow)]
struct Stored {
    id: String,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    synced_at: Option<chrono::DateTime<chrono::Utc>>,
    other_body: Option<String>,
}

struct PhonePurge {
    id: String,
    purged_at: chrono::DateTime<chrono::Utc>,
}

struct PhoneNote {
    id: String,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    synced_at: Option<chrono::DateTime<chrono::Utc>>,
    other_body: Option<String>,
}

type PgTx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;

fn is_newer(
    updated_at: chrono::DateTime<chrono::Utc>,
    synced_at: Option<chrono::DateTime<chrono::Utc>>,
) -> bool {
    match synced_at {
        None => true,
        Some(synced_at) => updated_at > synced_at,
    }
}

fn same_content(phone: &PhoneNote, row: &Stored) -> bool {
    phone.body == row.body && phone.deleted_at.is_some() == row.deleted_at.is_some()
}

fn agreed_times(phone: &PhoneNote, row: &Stored) -> bool {
    match row.synced_at {
        Some(synced_at) => {
            row.updated_at == synced_at
                && phone.synced_at == Some(synced_at)
                && phone.updated_at == synced_at
        }
        None => false,
    }
}

/// Postgres stores microseconds. Round so a time we bind matches the time we read back.
fn utc_micros(time: chrono::DateTime<chrono::FixedOffset>) -> chrono::DateTime<chrono::Utc> {
    let time = time.with_timezone(&chrono::Utc);
    let extra = time.timestamp_subsec_nanos() % 1_000;
    let down = time - chrono::Duration::nanoseconds(extra as i64);
    if extra >= 500 {
        down + chrono::Duration::microseconds(1)
    } else {
        down
    }
}

fn parse_time(
    value: Option<&serde_json::Value>,
    required: bool,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, ApiError> {
    match value {
        None | Some(serde_json::Value::Null) => {
            if required {
                Err(ApiError::BadTime)
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::String(text)) => {
            let parsed =
                chrono::DateTime::parse_from_rfc3339(text).map_err(|_| ApiError::BadTime)?;
            Ok(Some(utc_micros(parsed)))
        }
        Some(_) => Err(ApiError::BadTime),
    }
}

fn parse_note(value: &serde_json::Value) -> Result<PhoneNote, ApiError> {
    let Some(note) = value.as_object() else {
        return Err(ApiError::BadTime);
    };
    let id = match note.get("id") {
        Some(serde_json::Value::String(id)) => {
            let id = id.trim();
            if !is_uuid(id) {
                return Err(ApiError::BadId);
            }
            id.to_string()
        }
        _ => return Err(ApiError::BadId),
    };
    let body = match note.get("body") {
        Some(serde_json::Value::String(body)) => trimmed_body(body)?.to_string(),
        _ => return Err(ApiError::EmptyBody),
    };
    let created_at = parse_time(note.get("created_at"), true)?.ok_or(ApiError::BadTime)?;
    let updated_at = parse_time(note.get("updated_at"), true)?.ok_or(ApiError::BadTime)?;
    let deleted_at = parse_time(note.get("deleted_at"), false)?;
    let synced_at = parse_time(note.get("synced_at"), false)?;
    let other_body = match note.get("other_body") {
        Some(serde_json::Value::String(body)) => Some(body.clone()),
        _ => None,
    };
    Ok(PhoneNote {
        id,
        body,
        created_at,
        updated_at,
        deleted_at,
        synced_at,
        other_body,
    })
}

fn parse_sync(value: &serde_json::Value) -> Result<(Vec<PhoneNote>, Vec<PhonePurge>), ApiError> {
    let Some(root) = value.as_object() else {
        return Err(ApiError::BadTime);
    };
    let mut notes = Vec::new();
    let mut note_at = std::collections::HashMap::new();
    if let Some(entries) = root.get("entries") {
        let Some(entries) = entries.as_array() else {
            return Err(ApiError::BadTime);
        };
        for entry in entries {
            let note = parse_note(entry)?;
            if let Some(index) = note_at.get(&note.id) {
                notes[*index] = note;
            } else {
                note_at.insert(note.id.clone(), notes.len());
                notes.push(note);
            }
        }
    }
    let mut purges = Vec::new();
    let mut seen_purges = std::collections::HashSet::new();
    if let Some(rows) = root.get("purges") {
        let Some(rows) = rows.as_array() else {
            return Err(ApiError::BadTime);
        };
        for row in rows {
            let Some(row) = row.as_object() else {
                return Err(ApiError::BadTime);
            };
            let id = match row.get("id") {
                Some(serde_json::Value::String(id)) => {
                    let id = id.trim();
                    if !is_uuid(id) {
                        return Err(ApiError::BadId);
                    }
                    id.to_string()
                }
                _ => return Err(ApiError::BadId),
            };
            let purged_at = parse_time(row.get("purged_at"), true)?.ok_or(ApiError::BadTime)?;
            if seen_purges.insert(id.clone()) {
                purges.push(PhonePurge { id, purged_at });
            }
        }
    }
    Ok((notes, purges))
}

fn for_phone(note: &PhoneNote, row: &Stored) -> SyncEntry {
    SyncEntry {
        id: note.id.clone(),
        body: row.body.clone(),
        created_at: note.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
        synced_at: row.synced_at,
        other_body: row.other_body.clone(),
    }
}

fn from_row(row: &Stored) -> SyncEntry {
    SyncEntry {
        id: row.id.clone(),
        body: row.body.clone(),
        created_at: row.created_at,
        updated_at: row.updated_at,
        deleted_at: row.deleted_at,
        synced_at: row.synced_at,
        other_body: row.other_body.clone(),
    }
}

async fn load_stored(tx: &mut PgTx<'_>, id: &str) -> Result<Option<Stored>, ApiError> {
    sqlx::query_as::<_, Stored>(
        "SELECT id, body, created_at, updated_at, deleted_at, synced_at, other_body
         FROM entries
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(ApiError::Db)
}

async fn stamp_phone(tx: &mut PgTx<'_>, note: &PhoneNote) -> Result<Stored, ApiError> {
    sqlx::query_as::<_, Stored>(
        "UPDATE entries
         SET body = $1,
             deleted_at = $2,
             updated_at = now(),
             synced_at = now(),
             other_body = NULL
         WHERE id = $3
         RETURNING id, body, created_at, updated_at, deleted_at, synced_at, other_body",
    )
    .bind(&note.body)
    .bind(note.deleted_at)
    .bind(&note.id)
    .fetch_one(&mut **tx)
    .await
    .map_err(ApiError::Db)
}

async fn stamp_pc(tx: &mut PgTx<'_>, id: &str) -> Result<Stored, ApiError> {
    sqlx::query_as::<_, Stored>(
        "UPDATE entries
         SET updated_at = now(),
             synced_at = now(),
             other_body = NULL
         WHERE id = $1
         RETURNING id, body, created_at, updated_at, deleted_at, synced_at, other_body",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(ApiError::Db)
}

async fn clear_other(tx: &mut PgTx<'_>, id: &str) -> Result<Stored, ApiError> {
    sqlx::query_as::<_, Stored>(
        "UPDATE entries
         SET other_body = NULL
         WHERE id = $1
         RETURNING id, body, created_at, updated_at, deleted_at, synced_at, other_body",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(ApiError::Db)
}

async fn store_other(tx: &mut PgTx<'_>, id: &str, other_body: &str) -> Result<(), ApiError> {
    sqlx::query("UPDATE entries SET other_body = $1 WHERE id = $2")
        .bind(other_body)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(ApiError::Db)?;
    Ok(())
}

async fn merge_note(
    tx: &mut PgTx<'_>,
    note: &PhoneNote,
    row: Stored,
) -> Result<Option<SyncEntry>, ApiError> {
    let phone_newer = is_newer(note.updated_at, note.synced_at);
    let pc_newer = is_newer(row.updated_at, row.synced_at);
    if phone_newer && !pc_newer {
        let stored = stamp_phone(tx, note).await?;
        return Ok(Some(for_phone(note, &stored)));
    }
    if pc_newer && !phone_newer {
        let stored = stamp_pc(tx, &note.id).await?;
        return Ok(Some(for_phone(note, &stored)));
    }
    if same_content(note, &row) {
        if agreed_times(note, &row) && row.other_body.is_none() && note.other_body.is_none() {
            return Ok(None);
        }
        if agreed_times(note, &row) {
            let stored = clear_other(tx, &note.id).await?;
            return Ok(Some(for_phone(note, &stored)));
        }
        let stored = stamp_pc(tx, &note.id).await?;
        return Ok(Some(for_phone(note, &stored)));
    }
    store_other(tx, &note.id, &note.body).await?;
    Ok(Some(SyncEntry {
        id: note.id.clone(),
        body: note.body.clone(),
        created_at: note.created_at,
        updated_at: note.updated_at,
        deleted_at: note.deleted_at,
        synced_at: row.synced_at,
        other_body: Some(row.body),
    }))
}

async fn apply_sync(
    tx: &mut PgTx<'_>,
    notes: Vec<PhoneNote>,
    purges: Vec<PhonePurge>,
) -> Result<SyncPayload, ApiError> {
    let mut entries = Vec::new();
    let mut out_purges = Vec::new();
    let mut seen = Vec::new();
    let mut blocked = std::collections::HashSet::new();
    let mut echoed = std::collections::HashSet::new();

    for purge in purges {
        seen.push(purge.id.clone());
        let declined = sqlx::query_scalar::<_, bool>(
            "SELECT COALESCE(declined_purged_at = $2, false)
             FROM entries
             WHERE id = $1",
        )
        .bind(&purge.id)
        .bind(purge.purged_at)
        .fetch_optional(&mut **tx)
        .await
        .map_err(ApiError::Db)?;
        if declined == Some(true) {
            if let Some(row) = load_stored(tx, &purge.id).await? {
                entries.push(from_row(&row));
                echoed.insert(purge.id);
            }
            continue;
        }
        sqlx::query(
            "INSERT INTO purges (id, purged_at)
             VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(&purge.id)
        .bind(purge.purged_at)
        .execute(&mut **tx)
        .await
        .map_err(ApiError::Db)?;
        sqlx::query(
            "UPDATE entries
             SET declined_purged_at = NULL
             WHERE id = $1 AND declined_purged_at IS NOT NULL",
        )
        .bind(&purge.id)
        .execute(&mut **tx)
        .await
        .map_err(ApiError::Db)?;
        blocked.insert(purge.id);
    }

    for note in &notes {
        if !seen.iter().any(|id| id == &note.id) {
            seen.push(note.id.clone());
        }
        if echoed.contains(&note.id) {
            continue;
        }
        if blocked.contains(&note.id) {
            if let Some(purge) =
                sqlx::query_as::<_, SyncPurge>("SELECT id, purged_at FROM purges WHERE id = $1")
                    .bind(&note.id)
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(ApiError::Db)?
            {
                out_purges.push(purge);
            }
            continue;
        }
        let existing_purge =
            sqlx::query_as::<_, SyncPurge>("SELECT id, purged_at FROM purges WHERE id = $1")
                .bind(&note.id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(ApiError::Db)?;
        if let Some(purge) = existing_purge {
            out_purges.push(purge);
            continue;
        }
        if let Some(row) = load_stored(tx, &note.id).await? {
            if let Some(entry) = merge_note(tx, note, row).await? {
                entries.push(entry);
            }
            continue;
        }
        let stored = sqlx::query_as::<_, Stored>(
            "INSERT INTO entries (id, body, created_at, updated_at, deleted_at, synced_at)
             VALUES ($1, $2, $3, now(), $4, now())
             RETURNING id, body, created_at, updated_at, deleted_at, synced_at, other_body",
        )
        .bind(&note.id)
        .bind(&note.body)
        .bind(note.created_at)
        .bind(note.deleted_at)
        .fetch_one(&mut **tx)
        .await
        .map_err(ApiError::Db)?;
        entries.push(for_phone(note, &stored));
    }

    let pc_only = sqlx::query_as::<_, Stored>(
        "UPDATE entries
         SET updated_at = now(),
             synced_at = now(),
             other_body = NULL
         WHERE NOT EXISTS (SELECT 1 FROM purges WHERE purges.id = entries.id)
           AND id <> ALL($1::text[])
         RETURNING id, body, created_at, updated_at, deleted_at, synced_at, other_body",
    )
    .bind(&seen)
    .fetch_all(&mut **tx)
    .await
    .map_err(ApiError::Db)?;
    for row in pc_only {
        entries.push(from_row(&row));
    }

    Ok(SyncPayload {
        entries,
        purges: out_purges,
    })
}

#[utoipa::path(
    post,
    path = "/api/sync",
    request_body = SyncPayload,
    responses(
        (status = 200, description = "Notes for the phone to write, and purges for it to ask about", body = SyncPayload),
        (status = 400, description = "Bad id, empty body, or bad time", body = ErrorBody)
    )
)]
async fn sync_entries(
    axum::extract::State(state): axum::extract::State<AppState>,
    payload: Result<axum::Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> Result<axum::Json<SyncPayload>, ApiError> {
    let axum::Json(value) = payload.map_err(|_| ApiError::BadTime)?;
    let (notes, purges) = parse_sync(&value)?;
    let mut tx = state.pool.begin().await.map_err(ApiError::Db)?;
    let response = apply_sync(&mut tx, notes, purges).await?;
    tx.commit().await.map_err(ApiError::Db)?;
    Ok(axum::Json(response))
}

#[utoipa::path(
    get,
    path = "/api/purges/pending",
    responses((status = 200, description = "Purges waiting for an answer", body = PendingPurgeList))
)]
async fn list_pending_purges(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<axum::Json<PendingPurgeList>, ApiError> {
    let purges = sqlx::query_as::<_, PendingPurge>(
        "SELECT p.id,
                p.purged_at,
                (e.synced_at IS NULL OR e.updated_at > e.synced_at) AS changed
         FROM purges p
         INNER JOIN entries e ON e.id = p.id
         ORDER BY p.purged_at, p.id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(ApiError::Db)?;
    Ok(axum::Json(PendingPurgeList { purges }))
}

#[utoipa::path(
    post,
    path = "/api/purges/{id}/accept",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 204, description = "Entry removed, purge kept"),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
async fn accept_purge(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, ApiError> {
    let mut tx = state.pool.begin().await.map_err(ApiError::Db)?;
    let found = sqlx::query("SELECT 1 FROM purges WHERE id = $1")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::Db)?;
    if found.is_none() {
        return Err(ApiError::NotFound);
    }
    sqlx::query("DELETE FROM entries WHERE id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Db)?;
    tx.commit().await.map_err(ApiError::Db)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/purges/{id}/decline",
    params(("id" = String, Path, description = "Entry id")),
    request_body(content = SyncEntry, description = "Sync entry when the row is already gone", content_type = "application/json"),
    responses(
        (status = 200, description = "Purge removed and the entry kept", body = Entry),
        (status = 400, description = "Bad id, empty body, or bad time", body = ErrorBody),
        (status = 404, description = "Not found", body = ErrorBody)
    )
)]
async fn decline_purge(
    axum::extract::State(state): axum::extract::State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    body: axum::body::Bytes,
) -> Result<axum::Json<Entry>, ApiError> {
    let mut tx = state.pool.begin().await.map_err(ApiError::Db)?;
    let purge = sqlx::query_as::<_, SyncPurge>("SELECT id, purged_at FROM purges WHERE id = $1")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(ApiError::Db)?;
    let Some(purge) = purge else {
        return Err(ApiError::NotFound);
    };
    let existing = sqlx::query_as::<_, Entry>(
        "UPDATE entries
         SET declined_purged_at = $2
         WHERE id = $1
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(&id)
    .bind(purge.purged_at)
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Db)?;
    if let Some(entry) = existing {
        sqlx::query("DELETE FROM purges WHERE id = $1")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Db)?;
        tx.commit().await.map_err(ApiError::Db)?;
        return Ok(axum::Json(entry));
    }
    let value = if body.is_empty() {
        return Err(ApiError::BadId);
    } else {
        serde_json::from_slice(&body).map_err(|_| ApiError::BadTime)?
    };
    let note = parse_note(&value)?;
    if note.id != id {
        return Err(ApiError::BadId);
    }
    let entry = sqlx::query_as::<_, Entry>(
        "INSERT INTO entries (
             id, body, created_at, updated_at, deleted_at, synced_at, other_body, declined_purged_at
         )
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(&note.id)
    .bind(&note.body)
    .bind(note.created_at)
    .bind(note.updated_at)
    .bind(note.deleted_at)
    .bind(note.synced_at)
    .bind(&note.other_body)
    .bind(purge.purged_at)
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Db)?;
    sqlx::query("DELETE FROM purges WHERE id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Db)?;
    tx.commit().await.map_err(ApiError::Db)?;
    Ok(axum::Json(entry))
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
        purge_entry,
        sync_entries,
        list_pending_purges,
        accept_purge,
        decline_purge
    ),
    components(schemas(
        Entry,
        EntryList,
        NewEntry,
        EntryUpdate,
        ErrorBody,
        SyncEntry,
        SyncPurge,
        SyncPayload,
        PendingPurge,
        PendingPurgeList
    ))
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
        .route("/api/sync", axum::routing::post(sync_entries))
        .route(
            "/api/purges/pending",
            axum::routing::get(list_pending_purges),
        )
        .route("/api/purges/{id}/accept", axum::routing::post(accept_purge))
        .route(
            "/api/purges/{id}/decline",
            axum::routing::post(decline_purge),
        )
        .with_state(AppState { pool })
}
