use axum::{Router, extract::State, http::StatusCode, routing::get};
use sqlx::PgPool;

use crate::AppState;

pub fn router() -> Router<AppState> {
	Router::new()
		.route("/healthz", get(liveness_handler))
		.route("/readyz", get(readiness_handler))
}

async fn liveness_handler() -> StatusCode {
	StatusCode::NO_CONTENT
}

async fn readiness_handler(State(db_pool): State<PgPool>) -> StatusCode {
	match sqlx::query("SELECT 1").execute(&db_pool).await {
		Ok(_) => StatusCode::NO_CONTENT,
		Err(error) => {
			tracing::warn!(%error, "readiness check failed");
			StatusCode::SERVICE_UNAVAILABLE
		}
	}
}
