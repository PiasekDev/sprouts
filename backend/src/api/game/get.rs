use axum::{
	Json,
	extract::{Path, State},
};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::{GameResponse, fetch_game_for_user};
use crate::api::auth::session::CurrentUser;
use crate::api::support::error::AppError;
use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};

pub async fn handler(
	State(db_pool): State<PgPool>,
	current_user: CurrentUser,
	Path(game_id): Path<Uuid>,
) -> Result<Json<GameResponse>, AppError> {
	let game = fetch_game_for_user(&db_pool, game_id, &current_user)
		.await?
		.ok_or(GameNotFound)?;

	Ok(Json(game))
}

#[derive(Debug, Error)]
#[error("game not found")]
struct GameNotFound;

impl From<GameNotFound> for AppError {
	fn from(error: GameNotFound) -> Self {
		ProblemDetails::from(error).into()
	}
}

impl From<GameNotFound> for ProblemDetails {
	fn from(_: GameNotFound) -> Self {
		ProblemDetails::new(ProblemType::Custom("game-not-found"))
			.with_status(axum::http::StatusCode::NOT_FOUND)
			.with_title("Game not found")
			.with_detail("The requested game does not exist or is not accessible.")
			.with_error(ProblemField::new(
				"#",
				"game_not_found",
				"The requested game does not exist or is not accessible.",
			))
	}
}
