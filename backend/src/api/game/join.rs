use axum::{
	Json,
	extract::{Path, State},
	http::StatusCode,
};
use color_eyre::eyre::{OptionExt, WrapErr};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::{GameResponse, fetch_game_for_user};
use crate::AppState;
use crate::api::auth::session::CurrentUser;
use crate::api::support::error::AppError;
use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::domain::game::GameStatus;

pub async fn handler(
	State(app_state): State<AppState>,
	current_user: CurrentUser,
	Path(game_id): Path<Uuid>,
) -> Result<Json<GameResponse>, AppError> {
	let game = fetch_game_summary(&app_state.db_pool, game_id)
		.await
		.wrap_err("failed to fetch game summary during join")?
		.ok_or(JoinGameError::GameNotFound)?;

	if game.player1_user_id == current_user.id {
		return Err(JoinGameError::CannotJoinOwnGame.into());
	}

	if game.player2_user_id.is_some() || game.status != GameStatus::Waiting {
		return Err(JoinGameError::GameNotJoinable.into());
	}

	let rows_affected = sqlx::query!(
		r#"
		UPDATE games
		SET
			player2_user_id = $2,
			current_turn_user_id = player1_user_id,
			status = $3,
			updated_at = NOW()
		WHERE id = $1
			AND player2_user_id IS NULL
			AND status = $4
		"#,
		game.id,
		current_user.id,
		GameStatus::Active as GameStatus,
		GameStatus::Waiting as GameStatus,
	)
	.execute(&app_state.db_pool)
	.await
	.wrap_err("failed to join game")?
	.rows_affected();

	if rows_affected == 0 {
		return Err(JoinGameError::GameNotJoinable.into());
	}

	let game = fetch_game_for_user(&app_state.db_pool, game_id, &current_user)
		.await
		.wrap_err("failed to fetch joined game")?
		.ok_or_eyre("joined game could not be fetched for the joining player")?;

	Ok(Json(game))
}

struct GameSummary {
	id: Uuid,
	status: GameStatus,
	player1_user_id: Uuid,
	player2_user_id: Option<Uuid>,
}

async fn fetch_game_summary(
	db_pool: &PgPool,
	game_id: Uuid,
) -> color_eyre::Result<Option<GameSummary>> {
	let game = sqlx::query_as!(
		GameSummary,
		r#"
		SELECT
			id,
			status AS "status!: GameStatus",
			player1_user_id,
			player2_user_id
		FROM games
		WHERE id = $1
		"#,
		game_id,
	)
	.fetch_optional(db_pool)
	.await?;

	Ok(game)
}

#[derive(Debug, Error)]
enum JoinGameError {
	#[error("game not found")]
	GameNotFound,

	#[error("cannot join your own game")]
	CannotJoinOwnGame,

	#[error("game is not joinable")]
	GameNotJoinable,
}

impl From<JoinGameError> for AppError {
	fn from(error: JoinGameError) -> Self {
		ProblemDetails::from(error).into()
	}
}

impl From<JoinGameError> for ProblemDetails {
	fn from(error: JoinGameError) -> Self {
		match error {
			JoinGameError::GameNotFound => {
				ProblemDetails::new(ProblemType::Custom("game-not-found"))
					.with_status(StatusCode::NOT_FOUND)
					.with_title("Game not found")
					.with_detail("The requested game does not exist.")
					.with_error(ProblemField::new(
						"#",
						"game_not_found",
						"The requested game does not exist.",
					))
			}
			JoinGameError::CannotJoinOwnGame => {
				ProblemDetails::new(ProblemType::Custom("cannot-join-own-game"))
					.with_status(StatusCode::CONFLICT)
					.with_title("Cannot join your own game")
					.with_detail("You cannot join a game that you created.")
					.with_error(ProblemField::new(
						"#",
						"cannot_join_own_game",
						"You cannot join a game that you created.",
					))
			}
			JoinGameError::GameNotJoinable => {
				ProblemDetails::new(ProblemType::Custom("game-not-joinable"))
					.with_status(StatusCode::CONFLICT)
					.with_title("Game is not joinable")
					.with_detail(
						"The requested game already has two players or is no longer waiting.",
					)
					.with_error(ProblemField::new(
						"#",
						"game_not_joinable",
						"The requested game already has two players or is no longer waiting.",
					))
			}
		}
	}
}
