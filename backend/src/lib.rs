use crate::config::AppConfig;
use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;

pub mod config;

pub mod api {
	use crate::AppState;
	use axum::Router;

	pub mod auth;
	pub mod game;

	pub mod support {
		pub mod error;
		pub mod problem;
		pub mod validation;
	}

	pub fn router() -> Router<AppState> {
		Router::new()
			.nest("/auth", auth::router())
			.nest("/game", game::router())
	}
}

pub mod domain {
	pub mod game;
	pub mod password_hash;
	pub mod plain_password;
	pub mod session;
	pub mod username;
}

#[derive(Clone)]
pub struct AppState {
	pub db_pool: PgPool,
	pub config: Arc<AppConfig>,
}

pub fn app(state: AppState) -> Router {
	Router::new().nest("/api", api::router()).with_state(state)
}
