#[derive(utoipa::OpenApi)]
#[openapi(
    paths(
        crate::entries::list_entries,
        crate::entries::list_deleted,
        crate::entries::create_entry,
        crate::entries::update_entry,
        crate::entries::delete_entry,
        crate::entries::restore_entry,
        crate::entries::purge_entry,
        crate::sync::sync_entries,
        crate::sync::list_pending_purges,
        crate::sync::accept_purge,
        crate::sync::decline_purge
    ),
    components(schemas(
        crate::entries::Entry,
        crate::entries::ListedEntry,
        crate::entries::EntryList,
        crate::entries::NewEntry,
        crate::entries::EntryUpdate,
        crate::error::ErrorBody,
        crate::sync::SyncEntry,
        crate::sync::SyncPurge,
        crate::sync::SyncPayload,
        crate::sync::PendingPurge,
        crate::sync::PendingPurgeList
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

pub(crate) fn routes<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
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
}
