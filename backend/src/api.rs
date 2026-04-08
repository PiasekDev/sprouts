use axum::Router;

pub mod auth;

pub fn router() -> Router {
	Router::new().nest("/auth", auth::router())
}
