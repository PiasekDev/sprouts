use axum::{Router, routing::post};

use crate::AppState;

pub mod login;
pub mod register;

pub fn router() -> Router<AppState> {
	Router::new()
		.route("/register", post(register::handler))
		.route("/login", post(login::handler))
}
