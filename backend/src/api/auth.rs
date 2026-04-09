use axum::{
	Router,
	routing::{get, post},
};

use crate::AppState;

pub mod login;
pub mod logout;
pub mod me;
pub mod register;
pub mod session;

pub fn router() -> Router<AppState> {
	Router::new()
		.route("/me", get(me::handler))
		.route("/register", post(register::handler))
		.route("/login", post(login::handler))
		.route("/logout", post(logout::handler))
}
