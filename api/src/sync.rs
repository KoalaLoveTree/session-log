#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct SyncEntry {
    id: String,
    body: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    synced_at: Option<chrono::DateTime<chrono::Utc>>,
    other_body: Option<String>,
}

#[derive(serde::Serialize, utoipa::ToSchema, sqlx::FromRow)]
pub(crate) struct SyncPurge {
    id: String,
    purged_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct SyncPayload {
    entries: Vec<SyncEntry>,
    purges: Vec<SyncPurge>,
}

#[derive(serde::Serialize, utoipa::ToSchema, sqlx::FromRow)]
pub(crate) struct PendingPurge {
    id: String,
    purged_at: chrono::DateTime<chrono::Utc>,
    changed: bool,
}

#[derive(serde::Serialize, utoipa::ToSchema)]
pub(crate) struct PendingPurgeList {
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

/// `Transaction` derefs to the connection. A helper's `&mut PgTx` runs queries on `&mut **tx`.
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
) -> Result<Option<chrono::DateTime<chrono::Utc>>, crate::error::ApiError> {
    match value {
        None | Some(serde_json::Value::Null) => {
            if required {
                Err(crate::error::ApiError::BadTime)
            } else {
                Ok(None)
            }
        }
        Some(serde_json::Value::String(text)) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(text)
                .map_err(|_| crate::error::ApiError::BadTime)?;
            Ok(Some(utc_micros(parsed)))
        }
        Some(_) => Err(crate::error::ApiError::BadTime),
    }
}

fn parse_note(value: &serde_json::Value) -> Result<PhoneNote, crate::error::ApiError> {
    let Some(note) = value.as_object() else {
        return Err(crate::error::ApiError::BadTime);
    };
    let id = match note.get("id") {
        Some(serde_json::Value::String(id)) => {
            let id = id.trim();
            if !crate::entries::is_uuid(id) {
                return Err(crate::error::ApiError::BadId);
            }
            id.to_string()
        }
        _ => return Err(crate::error::ApiError::BadId),
    };
    let body = match note.get("body") {
        Some(serde_json::Value::String(body)) => crate::entries::trimmed_body(body)?.to_string(),
        _ => return Err(crate::error::ApiError::EmptyBody),
    };
    let created_at =
        parse_time(note.get("created_at"), true)?.ok_or(crate::error::ApiError::BadTime)?;
    let updated_at =
        parse_time(note.get("updated_at"), true)?.ok_or(crate::error::ApiError::BadTime)?;
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

fn parse_sync(
    value: &serde_json::Value,
) -> Result<(Vec<PhoneNote>, Vec<PhonePurge>), crate::error::ApiError> {
    let Some(root) = value.as_object() else {
        return Err(crate::error::ApiError::BadTime);
    };
    let mut notes = Vec::new();
    let mut note_at = std::collections::HashMap::new();
    if let Some(entries) = root.get("entries") {
        let Some(entries) = entries.as_array() else {
            return Err(crate::error::ApiError::BadTime);
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
            return Err(crate::error::ApiError::BadTime);
        };
        for row in rows {
            let Some(row) = row.as_object() else {
                return Err(crate::error::ApiError::BadTime);
            };
            let id = match row.get("id") {
                Some(serde_json::Value::String(id)) => {
                    let id = id.trim();
                    if !crate::entries::is_uuid(id) {
                        return Err(crate::error::ApiError::BadId);
                    }
                    id.to_string()
                }
                _ => return Err(crate::error::ApiError::BadId),
            };
            let purged_at =
                parse_time(row.get("purged_at"), true)?.ok_or(crate::error::ApiError::BadTime)?;
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

async fn load_stored(
    tx: &mut PgTx<'_>,
    id: &str,
) -> Result<Option<Stored>, crate::error::ApiError> {
    sqlx::query_as::<_, Stored>(
        "SELECT id, body, created_at, updated_at, deleted_at, synced_at, other_body
         FROM entries
         WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(crate::error::ApiError::Db)
}

async fn stamp_phone(
    tx: &mut PgTx<'_>,
    note: &PhoneNote,
) -> Result<Stored, crate::error::ApiError> {
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
    .map_err(crate::error::ApiError::Db)
}

async fn stamp_pc(tx: &mut PgTx<'_>, id: &str) -> Result<Stored, crate::error::ApiError> {
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
    .map_err(crate::error::ApiError::Db)
}

async fn clear_other(tx: &mut PgTx<'_>, id: &str) -> Result<Stored, crate::error::ApiError> {
    sqlx::query_as::<_, Stored>(
        "UPDATE entries
         SET other_body = NULL
         WHERE id = $1
         RETURNING id, body, created_at, updated_at, deleted_at, synced_at, other_body",
    )
    .bind(id)
    .fetch_one(&mut **tx)
    .await
    .map_err(crate::error::ApiError::Db)
}

async fn store_other(
    tx: &mut PgTx<'_>,
    id: &str,
    other_body: &str,
) -> Result<(), crate::error::ApiError> {
    sqlx::query("UPDATE entries SET other_body = $1 WHERE id = $2")
        .bind(other_body)
        .bind(id)
        .execute(&mut **tx)
        .await
        .map_err(crate::error::ApiError::Db)?;
    Ok(())
}

async fn merge_note(
    tx: &mut PgTx<'_>,
    note: &PhoneNote,
    row: Stored,
) -> Result<Option<SyncEntry>, crate::error::ApiError> {
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

async fn load_purge(
    tx: &mut PgTx<'_>,
    id: &str,
) -> Result<Option<SyncPurge>, crate::error::ApiError> {
    sqlx::query_as::<_, SyncPurge>("SELECT id, purged_at FROM purges WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(crate::error::ApiError::Db)
}

async fn apply_sync(
    tx: &mut PgTx<'_>,
    notes: Vec<PhoneNote>,
    purges: Vec<PhonePurge>,
) -> Result<SyncPayload, crate::error::ApiError> {
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
        .map_err(crate::error::ApiError::Db)?;
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
        .map_err(crate::error::ApiError::Db)?;
        sqlx::query(
            "UPDATE entries
             SET declined_purged_at = NULL
             WHERE id = $1 AND declined_purged_at IS NOT NULL",
        )
        .bind(&purge.id)
        .execute(&mut **tx)
        .await
        .map_err(crate::error::ApiError::Db)?;
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
            if let Some(purge) = load_purge(tx, &note.id).await? {
                out_purges.push(purge);
            }
            continue;
        }
        if let Some(purge) = load_purge(tx, &note.id).await? {
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
        .map_err(crate::error::ApiError::Db)?;
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
    .map_err(crate::error::ApiError::Db)?;
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
        (status = 400, description = "Bad id, empty body, or bad time", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn sync_entries(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    payload: Result<axum::Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> Result<axum::Json<SyncPayload>, crate::error::ApiError> {
    let axum::Json(value) = payload.map_err(|_| crate::error::ApiError::BadTime)?;
    let (notes, purges) = parse_sync(&value)?;
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(crate::error::ApiError::Db)?;
    let response = apply_sync(&mut tx, notes, purges).await?;
    tx.commit().await.map_err(crate::error::ApiError::Db)?;
    Ok(axum::Json(response))
}

#[utoipa::path(
    get,
    path = "/api/purges/pending",
    responses((status = 200, description = "Purges waiting for an answer", body = PendingPurgeList))
)]
pub(crate) async fn list_pending_purges(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
) -> Result<axum::Json<PendingPurgeList>, crate::error::ApiError> {
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
    .map_err(crate::error::ApiError::Db)?;
    Ok(axum::Json(PendingPurgeList { purges }))
}

#[utoipa::path(
    post,
    path = "/api/purges/{id}/accept",
    params(("id" = String, Path, description = "Entry id")),
    responses(
        (status = 204, description = "Entry removed, purge kept"),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn accept_purge(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, crate::error::ApiError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(crate::error::ApiError::Db)?;
    let found = sqlx::query("SELECT 1 FROM purges WHERE id = $1")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(crate::error::ApiError::Db)?;
    if found.is_none() {
        return Err(crate::error::ApiError::NotFound);
    }
    sqlx::query("DELETE FROM entries WHERE id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(crate::error::ApiError::Db)?;
    tx.commit().await.map_err(crate::error::ApiError::Db)?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/purges/{id}/decline",
    params(("id" = String, Path, description = "Entry id")),
    request_body(content = SyncEntry, description = "Sync entry when the row is already gone", content_type = "application/json"),
    responses(
        (status = 200, description = "Purge removed and the entry kept", body = crate::entries::Entry),
        (status = 400, description = "Bad id, empty body, or bad time", body = crate::error::ErrorBody),
        (status = 404, description = "Not found", body = crate::error::ErrorBody)
    )
)]
pub(crate) async fn decline_purge(
    axum::extract::State(state): axum::extract::State<crate::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    body: axum::body::Bytes,
) -> Result<axum::Json<crate::entries::Entry>, crate::error::ApiError> {
    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(crate::error::ApiError::Db)?;
    let purge = load_purge(&mut tx, &id).await?;
    let Some(purge) = purge else {
        return Err(crate::error::ApiError::NotFound);
    };
    let existing = sqlx::query_as::<_, crate::entries::Entry>(
        "UPDATE entries
         SET declined_purged_at = $2
         WHERE id = $1
         RETURNING id, body, created_at, updated_at, deleted_at",
    )
    .bind(&id)
    .bind(purge.purged_at)
    .fetch_optional(&mut *tx)
    .await
    .map_err(crate::error::ApiError::Db)?;
    if let Some(entry) = existing {
        sqlx::query("DELETE FROM purges WHERE id = $1")
            .bind(&id)
            .execute(&mut *tx)
            .await
            .map_err(crate::error::ApiError::Db)?;
        tx.commit().await.map_err(crate::error::ApiError::Db)?;
        return Ok(axum::Json(entry));
    }
    let value = if body.is_empty() {
        return Err(crate::error::ApiError::BadId);
    } else {
        serde_json::from_slice(&body).map_err(|_| crate::error::ApiError::BadTime)?
    };
    let note = parse_note(&value)?;
    if note.id != id {
        return Err(crate::error::ApiError::BadId);
    }
    let entry = sqlx::query_as::<_, crate::entries::Entry>(
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
    .map_err(crate::error::ApiError::Db)?;
    sqlx::query("DELETE FROM purges WHERE id = $1")
        .bind(&id)
        .execute(&mut *tx)
        .await
        .map_err(crate::error::ApiError::Db)?;
    tx.commit().await.map_err(crate::error::ApiError::Db)?;
    Ok(axum::Json(entry))
}
