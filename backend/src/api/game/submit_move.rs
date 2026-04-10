use axum::{
	Json,
	extract::{Path, State},
	http::StatusCode,
};
use axum_extra::extract::WithRejection;
use color_eyre::eyre::{OptionExt, WrapErr};
use serde::Deserialize;
use serde_fields::SerdeField;
use thiserror::Error;
use uuid::Uuid;

use super::{GameResponse, fetch_game_for_user};
use crate::AppState;
use crate::api::auth::session::CurrentUser;
use crate::api::support::error::{AppError, RequestValidationError};
use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::domain::game::{BoardState, Edge, GameStatus, NewSpot, Spot, SubmittedMove};

pub async fn handler(
	State(app_state): State<AppState>,
	current_user: CurrentUser,
	Path(game_id): Path<Uuid>,
	WithRejection(Json(dto), _): WithRejection<Json<MoveRequestDto>, ProblemDetails>,
) -> Result<Json<GameResponse>, AppError> {
	let submitted_move = SubmittedMove::try_from(dto)?;
	submit_move(&app_state, game_id, &current_user, submitted_move).await?;
	let game = fetch_game_for_user(&app_state.db_pool, game_id, &current_user)
		.await
		.wrap_err("failed to fetch game after move submission")?
		.ok_or_eyre("updated game could not be fetched for the player who submitted a move")?;

	Ok(Json(game))
}

#[derive(Debug, Deserialize, SerdeField)]
pub struct MoveRequestDto {
	pub start_spot_id: i32,
	pub end_spot_id: i32,
	pub points: Vec<[f64; 2]>,
	pub new_spot: NewSpot,
}

impl TryFrom<MoveRequestDto> for SubmittedMove {
	type Error = RequestValidationError;

	fn try_from(dto: MoveRequestDto) -> Result<Self, Self::Error> {
		let MoveRequestDto {
			start_spot_id,
			end_spot_id,
			points,
			new_spot,
		} = dto;

		if start_spot_id <= 0 {
			return Err(RequestValidationError::from_errors(vec![
				ProblemField::for_field(
					MoveRequestDtoSerdeField::StartSpotId,
					"must_be_positive",
					"must be a positive integer",
				),
			]));
		}

		if end_spot_id <= 0 {
			return Err(RequestValidationError::from_errors(vec![
				ProblemField::for_field(
					MoveRequestDtoSerdeField::EndSpotId,
					"must_be_positive",
					"must be a positive integer",
				),
			]));
		}

		if points.len() < 2 {
			return Err(RequestValidationError::from_errors(vec![
				ProblemField::for_field(
					MoveRequestDtoSerdeField::Points,
					"too_short",
					"must contain at least 2 points",
				),
			]));
		}

		Ok(Self {
			start_spot_id,
			end_spot_id,
			points,
			new_spot,
		})
	}
}

async fn submit_move(
	app_state: &AppState,
	game_id: Uuid,
	current_user: &CurrentUser,
	submitted_move: SubmittedMove,
) -> color_eyre::Result<()> {
	let mut tx = app_state.db_pool.begin().await?;
	let game = fetch_game_context(&mut tx, game_id, current_user)
		.await?
		.ok_or(MoveError::GameNotFound)?;

	if game.status != GameStatus::Active {
		return Err(MoveError::GameNotActive.into());
	}

	let player2_user_id = game
		.player2_user_id
		.ok_or_eyre("active game did not contain a second player")?;
	let current_turn_user_id = game
		.current_turn_user_id
		.ok_or_eyre("active game did not contain current turn user id")?;

	if current_turn_user_id != current_user.id {
		return Err(MoveError::NotPlayersTurn.into());
	}

	let mut board_state = game.board_state_jsonb.0;
	apply_move(&mut board_state, &submitted_move)?;

	let move_number = board_state.edges.len() as i32;
	let next_turn_user_id = if current_user.id == game.player1_user_id {
		player2_user_id
	} else {
		game.player1_user_id
	};
	let path_jsonb =
		serde_json::to_value(&submitted_move.points).wrap_err("failed to serialize move path")?;
	let new_spot_jsonb = serde_json::to_value(&submitted_move.new_spot)
		.wrap_err("failed to serialize submitted move spot")?;
	let board_state_jsonb =
		serde_json::to_value(&board_state).wrap_err("failed to serialize updated board state")?;

	sqlx::query!(
		r#"
		INSERT INTO moves (
			game_id,
			player_user_id,
			move_number,
			start_spot_id,
			end_spot_id,
			path_jsonb,
			new_spot_jsonb
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7)
		"#,
		game_id,
		current_user.id,
		move_number,
		submitted_move.start_spot_id,
		submitted_move.end_spot_id,
		path_jsonb,
		new_spot_jsonb,
	)
	.execute(&mut *tx)
	.await?;

	sqlx::query!(
		r#"
		UPDATE games
		SET
			board_state_jsonb = $2,
			current_turn_user_id = $3,
			updated_at = NOW()
		WHERE id = $1
		"#,
		game_id,
		board_state_jsonb,
		next_turn_user_id,
	)
	.execute(&mut *tx)
	.await?;

	tx.commit().await?;

	Ok(())
}

struct GameMoveContext {
	status: GameStatus,
	player1_user_id: Uuid,
	player2_user_id: Option<Uuid>,
	current_turn_user_id: Option<Uuid>,
	board_state_jsonb: sqlx::types::Json<BoardState>,
}

async fn fetch_game_context(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	game_id: Uuid,
	current_user: &CurrentUser,
) -> color_eyre::Result<Option<GameMoveContext>> {
	let game = sqlx::query_as!(
		GameMoveContext,
		r#"
		SELECT
			status AS "status!: GameStatus",
			player1_user_id,
			player2_user_id,
			current_turn_user_id,
			board_state_jsonb AS "board_state_jsonb!: sqlx::types::Json<BoardState>"
		FROM games
		WHERE id = $1
			AND ($2 = player1_user_id OR $2 = player2_user_id)
		FOR UPDATE
		"#,
		game_id,
		current_user.id,
	)
	.fetch_optional(&mut **tx)
	.await
	.wrap_err("failed to fetch game context for move submission")?;

	Ok(game)
}

// This is the first validation/application pass from the MVP plan.
// It enforces basic game-state and capacity rules, but does not yet do
// geometric checks such as edge intersections, self-intersections, or
// improper touches against unrelated spots and edges.
fn apply_move(
	board_state: &mut BoardState,
	submitted_move: &SubmittedMove,
) -> Result<(), MoveError> {
	let start_spot_index = board_state
		.spots
		.iter()
		.position(|spot| spot.id == submitted_move.start_spot_id)
		.ok_or(MoveError::SpotNotFound {
			field: "start_spot_id",
			spot_id: submitted_move.start_spot_id,
		})?;
	let end_spot_index = board_state
		.spots
		.iter()
		.position(|spot| spot.id == submitted_move.end_spot_id)
		.ok_or(MoveError::SpotNotFound {
			field: "end_spot_id",
			spot_id: submitted_move.end_spot_id,
		})?;
	let start_spot = board_state.spots[start_spot_index].clone();
	let end_spot = board_state.spots[end_spot_index].clone();

	if is_same_point(&submitted_move.new_spot, &start_spot)
		|| is_same_point(&submitted_move.new_spot, &end_spot)
	{
		return Err(MoveError::NewSpotIsEndpoint);
	}

	if start_spot.id == end_spot.id {
		if start_spot.degree > 1 {
			return Err(MoveError::SpotCapacityExceeded {
				field: "start_spot_id",
				spot_id: start_spot.id,
			});
		}
	} else {
		if start_spot.degree > 2 {
			return Err(MoveError::SpotCapacityExceeded {
				field: "start_spot_id",
				spot_id: start_spot.id,
			});
		}

		if end_spot.degree > 2 {
			return Err(MoveError::SpotCapacityExceeded {
				field: "end_spot_id",
				spot_id: end_spot.id,
			});
		}
	}

	if start_spot_index == end_spot_index {
		board_state.spots[start_spot_index].degree += 2;
	} else {
		board_state.spots[start_spot_index].degree += 1;
		board_state.spots[end_spot_index].degree += 1;
	}

	let new_spot_id = board_state.next_spot_id();
	let edge_id = board_state.next_edge_id();
	board_state.spots.push(Spot {
		id: new_spot_id,
		x: submitted_move.new_spot.x,
		y: submitted_move.new_spot.y,
		degree: 2,
	});
	board_state.edges.push(Edge {
		id: edge_id,
		start_spot_id: submitted_move.start_spot_id,
		end_spot_id: submitted_move.end_spot_id,
		points: submitted_move.points.clone(),
		new_spot_id,
	});

	Ok(())
}

fn is_same_point(new_spot: &NewSpot, spot: &Spot) -> bool {
	new_spot.x == spot.x && new_spot.y == spot.y
}

#[derive(Debug, Error)]
enum MoveError {
	#[error("game not found")]
	GameNotFound,

	#[error("game is not active")]
	GameNotActive,

	#[error("it is not the player's turn")]
	NotPlayersTurn,

	#[error("spot referenced by {field} was not found")]
	SpotNotFound { field: &'static str, spot_id: i32 },

	#[error("new spot cannot be placed on an endpoint")]
	NewSpotIsEndpoint,

	#[error("spot referenced by {field} has no remaining capacity")]
	SpotCapacityExceeded { field: &'static str, spot_id: i32 },
}

impl From<MoveError> for AppError {
	fn from(error: MoveError) -> Self {
		ProblemDetails::from(error).into()
	}
}

impl From<MoveError> for ProblemDetails {
	fn from(error: MoveError) -> Self {
		match error {
			MoveError::GameNotFound => ProblemDetails::new(ProblemType::Custom("game-not-found"))
				.with_status(StatusCode::NOT_FOUND)
				.with_title("Game not found")
				.with_detail("The requested game does not exist or is not accessible.")
				.with_error(ProblemField::new(
					"#",
					"game_not_found",
					"The requested game does not exist or is not accessible.",
				)),
			MoveError::GameNotActive => ProblemDetails::new(ProblemType::Custom("game-not-active"))
				.with_status(StatusCode::CONFLICT)
				.with_title("Game is not active")
				.with_detail("Moves can only be submitted for an active game.")
				.with_error(ProblemField::new(
					"#",
					"game_not_active",
					"Moves can only be submitted for an active game.",
				)),
			MoveError::NotPlayersTurn => {
				ProblemDetails::new(ProblemType::Custom("not-players-turn"))
					.with_status(StatusCode::CONFLICT)
					.with_title("It is not your turn")
					.with_detail("Only the player whose turn it is can submit a move.")
					.with_error(ProblemField::new(
						"#",
						"not_players_turn",
						"Only the player whose turn it is can submit a move.",
					))
			}
			MoveError::SpotNotFound { field, spot_id } => {
				ProblemDetails::new(ProblemType::Custom("spot-not-found"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Referenced spot was not found")
					.with_detail(format!("No spot with id {spot_id} exists on the board."))
					.with_error(ProblemField::for_field(
						field,
						"spot_not_found",
						format!("No spot with id {spot_id} exists on the board."),
					))
			}
			MoveError::NewSpotIsEndpoint => {
				ProblemDetails::new(ProblemType::Custom("new-spot-is-endpoint"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("New spot cannot be an endpoint")
					.with_detail(
						"The new spot must not be placed exactly on the start or end spot.",
					)
					.with_error(ProblemField::for_field(
						"new_spot",
						"new_spot_is_endpoint",
						"The new spot must not be placed exactly on the start or end spot.",
					))
			}
			MoveError::SpotCapacityExceeded { field, spot_id } => {
				ProblemDetails::new(ProblemType::Custom("spot-capacity-exceeded"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Spot has no remaining capacity")
					.with_detail(format!(
						"Spot {spot_id} cannot accept another line end under the Sprouts rules."
					))
					.with_error(ProblemField::for_field(
						field,
						"spot_capacity_exceeded",
						format!(
							"Spot {spot_id} cannot accept another line end under the Sprouts rules."
						),
					))
			}
		}
	}
}
