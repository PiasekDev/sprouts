use axum::{Router, routing::post};

use crate::AppState;

mod cookie;
pub mod login;
pub mod register;

pub fn router() -> Router<AppState> {
	Router::new()
		.route("/register", post(register::handler))
		.route("/login", post(login::handler))
}
