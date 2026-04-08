use axum::{Router, routing::post};

pub mod login;
pub mod register;

pub fn router() -> Router {
	Router::new()
		.route("/register", post(register::handler))
		.route("/login", post(login::handler))
}
