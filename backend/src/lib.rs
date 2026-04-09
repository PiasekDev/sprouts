use axum::Router;
use sqlx::PgPool;

pub mod api {
	use crate::AppState;
	use axum::Router;

	pub mod auth;

	pub mod support {
		pub mod error;
		pub mod problem;
		pub mod validation;
	}

	pub fn router() -> Router<AppState> {
		Router::new().nest("/auth", auth::router())
	}
}

pub mod domain {
	pub mod password_hash;
	pub mod plain_password;
	pub mod username;
}

#[derive(Clone)]
pub struct AppState {
	pub db_pool: PgPool,
}

pub fn app(state: AppState) -> Router {
	Router::new().nest("/api", api::router()).with_state(state)
}
