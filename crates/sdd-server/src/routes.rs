use axum::{routing::get, Router};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{handlers, state::SharedState};

pub fn create_router(state: SharedState) -> Router {
    Router::new()
        .route("/healthcheck", get(handlers::healthcheck))
        .route("/stats", get(handlers::get_stats))
        .route("/requirements", get(handlers::list_requirements))
        .route(
            "/requirements/{requirementId}",
            get(handlers::get_requirement),
        )
        .route("/annotations", get(handlers::list_annotations))
        .route("/tasks", get(handlers::list_tasks))
        .route(
            "/scan",
            get(handlers::get_scan_status).post(handlers::trigger_scan),
        )
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
