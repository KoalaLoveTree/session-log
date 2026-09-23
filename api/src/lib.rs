mod doc;
mod entries;
mod error;
mod sync;

#[derive(Clone)]
struct AppState {
    pool: sqlx::PgPool,
}

pub fn app(pool: sqlx::PgPool) -> axum::Router {
    axum::Router::new()
        .merge(doc::routes())
        .route(
            "/api/entries",
            axum::routing::get(entries::list_entries).post(entries::create_entry),
        )
        .route(
            "/api/entries/deleted",
            axum::routing::get(entries::list_deleted),
        )
        .route(
            "/api/entries/{id}",
            axum::routing::put(entries::update_entry).delete(entries::delete_entry),
        )
        .route(
            "/api/entries/{id}/restore",
            axum::routing::post(entries::restore_entry),
        )
        .route(
            "/api/entries/{id}/purge",
            axum::routing::delete(entries::purge_entry),
        )
        .route("/api/sync", axum::routing::post(sync::sync_entries))
        .route(
            "/api/purges/pending",
            axum::routing::get(sync::list_pending_purges),
        )
        .route(
            "/api/purges/{id}/accept",
            axum::routing::post(sync::accept_purge),
        )
        .route(
            "/api/purges/{id}/decline",
            axum::routing::post(sync::decline_purge),
        )
        .with_state(AppState { pool })
}
