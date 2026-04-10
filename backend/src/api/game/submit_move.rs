use axum::{
	Json,
	extract::{Path, State},
	http::StatusCode,
};
use axum_extra::extract::WithRejection;
use color_eyre::eyre::{OptionExt, WrapErr};
use shared::game::{MoveRequest, MoveRequestSerdeField};
use sqlx::PgPool;
use thiserror::Error;
use uuid::Uuid;

use super::{GameResponse, fetch_game_for_user};
use crate::api::auth::session::CurrentUser;
use crate::api::support::error::{AppError, RequestValidationError};
use crate::api::support::problem::{ProblemDetails, ProblemField, ProblemType};
use crate::domain::game::{BoardState, Edge, GameStatus, NewSpot, Spot, SubmittedMove};

pub async fn handler(
	State(db_pool): State<PgPool>,
	current_user: CurrentUser,
	Path(game_id): Path<Uuid>,
	WithRejection(Json(dto), _): WithRejection<Json<MoveRequest>, ProblemDetails>,
) -> Result<Json<GameResponse>, AppError> {
	let submitted_move = SubmittedMove::try_from(dto)?;
	submit_move(&db_pool, game_id, &current_user, submitted_move).await?;
	let game = fetch_game_for_user(&db_pool, game_id, &current_user)
		.await
		.wrap_err("failed to fetch game after move submission")?
		.ok_or_eyre("updated game could not be fetched for the player who submitted a move")?;

	Ok(Json(game))
}

impl TryFrom<MoveRequest> for SubmittedMove {
	type Error = RequestValidationError;

	fn try_from(dto: MoveRequest) -> Result<Self, Self::Error> {
		let MoveRequest {
			start_spot_id,
			end_spot_id,
			points,
			new_spot,
		} = dto;

		if start_spot_id <= 0 {
			return Err(RequestValidationError::from_errors(vec![
				ProblemField::for_field(
					MoveRequestSerdeField::StartSpotId,
					"must_be_positive",
					"must be a positive integer",
				),
			]));
		}

		if end_spot_id <= 0 {
			return Err(RequestValidationError::from_errors(vec![
				ProblemField::for_field(
					MoveRequestSerdeField::EndSpotId,
					"must_be_positive",
					"must be a positive integer",
				),
			]));
		}

		if points.len() < 2 {
			return Err(RequestValidationError::from_errors(vec![
				ProblemField::for_field(
					MoveRequestSerdeField::Points,
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
	db_pool: &PgPool,
	game_id: Uuid,
	current_user: &CurrentUser,
	submitted_move: SubmittedMove,
) -> color_eyre::Result<()> {
	let mut tx = db_pool.begin().await?;
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

// This is the pragmatic MVP validation/application pass.
// It enforces the basic game-state and capacity rules, and also validates the
// submitted polyline endpoints, spot touches, self-intersections, and
// intersections with existing edges. More exact geometry can be tightened
// later if needed, but the server is already authoritative over move shape.
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
	let start_point = [start_spot.x, start_spot.y];
	let end_point = [end_spot.x, end_spot.y];
	let new_spot_point = [submitted_move.new_spot.x, submitted_move.new_spot.y];
	let points = &submitted_move.points;

	if !point_eq(points[0], start_point) {
		return Err(MoveError::PathDoesNotStartAtStartSpot);
	}

	if !point_eq(*points.last().expect("validated non-empty path"), end_point) {
		return Err(MoveError::PathDoesNotEndAtEndSpot);
	}

	if is_same_point(&submitted_move.new_spot, &start_spot)
		|| is_same_point(&submitted_move.new_spot, &end_spot)
	{
		return Err(MoveError::NewSpotIsEndpoint);
	}

	validate_path_segments(points)?;
	validate_spot_touches(board_state, submitted_move, start_point, end_point)?;

	if !point_on_polyline(new_spot_point, points) {
		return Err(MoveError::NewSpotNotOnPath);
	}

	validate_self_intersections(points)?;
	validate_existing_edge_intersections(board_state, submitted_move, start_point, end_point)?;

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

	let edge_points = path_with_new_spot(points, new_spot_point)?;
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
		points: edge_points,
		new_spot_id,
	});

	Ok(())
}

const EPSILON: f64 = 1e-6;

fn is_same_point(new_spot: &NewSpot, spot: &Spot) -> bool {
	approx_eq(new_spot.x, spot.x) && approx_eq(new_spot.y, spot.y)
}

fn validate_path_segments(points: &[[f64; 2]]) -> Result<(), MoveError> {
	for window in points.windows(2) {
		if point_eq(window[0], window[1]) {
			return Err(MoveError::PathHasDegenerateSegment);
		}
	}

	Ok(())
}

fn validate_spot_touches(
	board_state: &BoardState,
	submitted_move: &SubmittedMove,
	start_point: [f64; 2],
	end_point: [f64; 2],
) -> Result<(), MoveError> {
	for spot in &board_state.spots {
		let spot_point = [spot.x, spot.y];

		for (segment_index, segment) in submitted_move.points.windows(2).enumerate() {
			if !point_on_segment(spot_point, segment[0], segment[1]) {
				continue;
			}

			let is_allowed_start_touch = spot.id == submitted_move.start_spot_id
				&& segment_index == 0
				&& point_eq(spot_point, start_point)
				&& point_eq(spot_point, segment[0]);
			let is_allowed_end_touch = spot.id == submitted_move.end_spot_id
				&& segment_index == submitted_move.points.len() - 2
				&& point_eq(spot_point, end_point)
				&& point_eq(spot_point, segment[1]);

			if !is_allowed_start_touch && !is_allowed_end_touch {
				return Err(MoveError::PathTouchesExistingSpot { spot_id: spot.id });
			}
		}
	}

	Ok(())
}

fn validate_self_intersections(points: &[[f64; 2]]) -> Result<(), MoveError> {
	let segment_count = points.len() - 1;
	let is_loop = point_eq(points[0], points[points.len() - 1]);

	for i in 0..segment_count {
		for j in (i + 1)..segment_count {
			if j == i + 1 {
				continue;
			}

			let shared_loop_endpoint = is_loop
				&& i == 0 && j == segment_count - 1
				&& point_eq(points[0], points[points.len() - 1]);
			let allowed_endpoint = shared_loop_endpoint.then_some(points[0]);

			if segments_intersect_improperly(
				points[i],
				points[i + 1],
				points[j],
				points[j + 1],
				allowed_endpoint,
			) {
				return Err(MoveError::PathSelfIntersects);
			}
		}
	}

	Ok(())
}

fn validate_existing_edge_intersections(
	board_state: &BoardState,
	submitted_move: &SubmittedMove,
	start_point: [f64; 2],
	end_point: [f64; 2],
) -> Result<(), MoveError> {
	for edge in &board_state.edges {
		for submitted_segment in submitted_move.points.windows(2) {
			for existing_segment in edge.points.windows(2) {
				if segments_intersect_improperly(
					submitted_segment[0],
					submitted_segment[1],
					existing_segment[0],
					existing_segment[1],
					None,
				) {
					let touches_start_spot = shared_endpoint_is_allowed(
						submitted_segment[0],
						submitted_segment[1],
						existing_segment[0],
						existing_segment[1],
						start_point,
					);
					let touches_end_spot = shared_endpoint_is_allowed(
						submitted_segment[0],
						submitted_segment[1],
						existing_segment[0],
						existing_segment[1],
						end_point,
					);

					if !touches_start_spot && !touches_end_spot {
						return Err(MoveError::PathIntersectsExistingEdge);
					}
				}
			}
		}
	}

	Ok(())
}

fn point_on_polyline(point: [f64; 2], points: &[[f64; 2]]) -> bool {
	points
		.windows(2)
		.any(|segment| point_on_segment(point, segment[0], segment[1]))
}

fn path_with_new_spot(
	points: &[[f64; 2]],
	new_spot_point: [f64; 2],
) -> Result<Vec<[f64; 2]>, MoveError> {
	if points.iter().any(|point| point_eq(*point, new_spot_point)) {
		return Ok(points.to_vec());
	}

	let mut edge_points = Vec::with_capacity(points.len() + 1);

	for (index, point) in points.iter().copied().enumerate() {
		edge_points.push(point);

		let Some(next_point) = points.get(index + 1).copied() else {
			continue;
		};

		if point_on_segment(new_spot_point, point, next_point) {
			edge_points.push(new_spot_point);
			edge_points.extend_from_slice(&points[(index + 1)..]);
			return Ok(edge_points);
		}
	}

	Err(MoveError::NewSpotNotOnPath)
}

fn shared_endpoint_is_allowed(
	a1: [f64; 2],
	a2: [f64; 2],
	b1: [f64; 2],
	b2: [f64; 2],
	allowed_point: [f64; 2],
) -> bool {
	let shared_endpoints = [a1, a2]
		.into_iter()
		.flat_map(|point_a| [b1, b2].into_iter().map(move |point_b| (point_a, point_b)));

	for (point_a, point_b) in shared_endpoints {
		if point_eq(point_a, point_b) && point_eq(point_a, allowed_point) {
			return true;
		}
	}

	false
}

fn segments_intersect_improperly(
	a1: [f64; 2],
	a2: [f64; 2],
	b1: [f64; 2],
	b2: [f64; 2],
	allowed_shared_endpoint: Option<[f64; 2]>,
) -> bool {
	if !segments_intersect(a1, a2, b1, b2) {
		return false;
	}

	if let Some(allowed_point) = allowed_shared_endpoint {
		let only_allowed_touch = shared_endpoint_is_allowed(a1, a2, b1, b2, allowed_point)
			&& !segments_overlap(a1, a2, b1, b2);

		if only_allowed_touch {
			return false;
		}
	}

	if segments_overlap(a1, a2, b1, b2) {
		return true;
	}

	let shared_endpoint = [a1, a2]
		.into_iter()
		.flat_map(|point_a| [b1, b2].into_iter().map(move |point_b| (point_a, point_b)))
		.any(|(point_a, point_b)| point_eq(point_a, point_b));

	!shared_endpoint
}

fn segments_intersect(a1: [f64; 2], a2: [f64; 2], b1: [f64; 2], b2: [f64; 2]) -> bool {
	let o1 = orientation(a1, a2, b1);
	let o2 = orientation(a1, a2, b2);
	let o3 = orientation(b1, b2, a1);
	let o4 = orientation(b1, b2, a2);

	if o1 != o2 && o3 != o4 {
		return true;
	}

	if o1 == 0 && point_on_segment(b1, a1, a2) {
		return true;
	}
	if o2 == 0 && point_on_segment(b2, a1, a2) {
		return true;
	}
	if o3 == 0 && point_on_segment(a1, b1, b2) {
		return true;
	}
	if o4 == 0 && point_on_segment(a2, b1, b2) {
		return true;
	}

	false
}

fn segments_overlap(a1: [f64; 2], a2: [f64; 2], b1: [f64; 2], b2: [f64; 2]) -> bool {
	orientation(a1, a2, b1) == 0
		&& orientation(a1, a2, b2) == 0
		&& (point_on_segment(a1, b1, b2)
			|| point_on_segment(a2, b1, b2)
			|| point_on_segment(b1, a1, a2)
			|| point_on_segment(b2, a1, a2))
}

fn point_on_segment(point: [f64; 2], segment_start: [f64; 2], segment_end: [f64; 2]) -> bool {
	if orientation(segment_start, segment_end, point) != 0 {
		return false;
	}

	point[0] <= segment_start[0].max(segment_end[0]) + EPSILON
		&& point[0] + EPSILON >= segment_start[0].min(segment_end[0])
		&& point[1] <= segment_start[1].max(segment_end[1]) + EPSILON
		&& point[1] + EPSILON >= segment_start[1].min(segment_end[1])
}

fn orientation(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> i8 {
	let cross = (b[1] - a[1]) * (c[0] - b[0]) - (b[0] - a[0]) * (c[1] - b[1]);

	if approx_eq(cross, 0.0) {
		0
	} else if cross > 0.0 {
		1
	} else {
		2
	}
}

fn approx_eq(left: f64, right: f64) -> bool {
	(left - right).abs() <= EPSILON
}

fn point_eq(left: [f64; 2], right: [f64; 2]) -> bool {
	approx_eq(left[0], right[0]) && approx_eq(left[1], right[1])
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

	#[error("path does not start at the selected start spot")]
	PathDoesNotStartAtStartSpot,

	#[error("path does not end at the selected end spot")]
	PathDoesNotEndAtEndSpot,

	#[error("path contains a degenerate segment")]
	PathHasDegenerateSegment,

	#[error("new spot must lie on the submitted path")]
	NewSpotNotOnPath,

	#[error("path self-intersects")]
	PathSelfIntersects,

	#[error("path intersects an existing edge")]
	PathIntersectsExistingEdge,

	#[error("path touches an existing spot illegally")]
	PathTouchesExistingSpot { spot_id: i32 },

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
			MoveError::PathDoesNotStartAtStartSpot => {
				ProblemDetails::new(ProblemType::Custom("path-does-not-start-at-start-spot"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Path does not start at the selected start spot")
					.with_detail("The first path point must match the selected start spot.")
					.with_error(ProblemField::for_field(
						"points",
						"path_does_not_start_at_start_spot",
						"The first path point must match the selected start spot.",
					))
			}
			MoveError::PathDoesNotEndAtEndSpot => {
				ProblemDetails::new(ProblemType::Custom("path-does-not-end-at-end-spot"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Path does not end at the selected end spot")
					.with_detail("The last path point must match the selected end spot.")
					.with_error(ProblemField::for_field(
						"points",
						"path_does_not_end_at_end_spot",
						"The last path point must match the selected end spot.",
					))
			}
			MoveError::PathHasDegenerateSegment => {
				ProblemDetails::new(ProblemType::Custom("path-has-degenerate-segment"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Path contains a degenerate segment")
					.with_detail("Adjacent path points must not be identical.")
					.with_error(ProblemField::for_field(
						"points",
						"path_has_degenerate_segment",
						"Adjacent path points must not be identical.",
					))
			}
			MoveError::NewSpotNotOnPath => {
				ProblemDetails::new(ProblemType::Custom("new-spot-not-on-path"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("New spot is not on the path")
					.with_detail("The new spot must lie on one of the submitted path segments.")
					.with_error(ProblemField::for_field(
						"new_spot",
						"new_spot_not_on_path",
						"The new spot must lie on one of the submitted path segments.",
					))
			}
			MoveError::PathSelfIntersects => {
				ProblemDetails::new(ProblemType::Custom("path-self-intersects"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Path self-intersects")
					.with_detail("The submitted path must not intersect itself.")
					.with_error(ProblemField::for_field(
						"points",
						"path_self_intersects",
						"The submitted path must not intersect itself.",
					))
			}
			MoveError::PathIntersectsExistingEdge => {
				ProblemDetails::new(ProblemType::Custom("path-intersects-existing-edge"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Path intersects an existing edge")
					.with_detail("The submitted path must not intersect an existing edge.")
					.with_error(ProblemField::for_field(
						"points",
						"path_intersects_existing_edge",
						"The submitted path must not intersect an existing edge.",
					))
			}
			MoveError::PathTouchesExistingSpot { spot_id } => {
				ProblemDetails::new(ProblemType::Custom("path-touches-existing-spot"))
					.with_status(StatusCode::UNPROCESSABLE_ENTITY)
					.with_title("Path touches an existing spot illegally")
					.with_detail(format!(
						"The submitted path touches spot {spot_id} at a disallowed location."
					))
					.with_error(ProblemField::for_field(
						"points",
						"path_touches_existing_spot",
						format!(
							"The submitted path touches spot {spot_id} at a disallowed location."
						),
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

#[cfg(test)]
mod tests {
	use super::{apply_move, point_eq};
	use crate::domain::game::{BoardState, SubmittedMove};

	#[test]
	fn apply_move_inserts_new_spot_into_edge_points() {
		let mut board_state = BoardState::initial();

		apply_move(
			&mut board_state,
			&SubmittedMove {
				start_spot_id: 1,
				end_spot_id: 2,
				points: vec![[100.0, 100.0], [300.0, 100.0]],
				new_spot: shared::game::NewSpot { x: 200.0, y: 100.0 },
			},
		)
		.expect("initial move should succeed");

		assert_eq!(
			board_state.edges[0].points,
			vec![[100.0, 100.0], [200.0, 100.0], [300.0, 100.0]]
		);
	}

	#[test]
	fn apply_move_allows_follow_up_move_from_inserted_spot() {
		let mut board_state = BoardState::initial();

		apply_move(
			&mut board_state,
			&SubmittedMove {
				start_spot_id: 1,
				end_spot_id: 2,
				points: vec![[100.0, 100.0], [300.0, 100.0]],
				new_spot: shared::game::NewSpot { x: 200.0, y: 100.0 },
			},
		)
		.expect("initial move should succeed");

		apply_move(
			&mut board_state,
			&SubmittedMove {
				start_spot_id: 3,
				end_spot_id: 2,
				points: vec![[200.0, 100.0], [240.0, 150.0], [300.0, 100.0]],
				new_spot: shared::game::NewSpot { x: 270.0, y: 125.0 },
			},
		)
		.expect("move from newly inserted spot should succeed");

		assert!(
			board_state.edges[1]
				.points
				.iter()
				.any(|point| point_eq(*point, [270.0, 125.0]))
		);
	}
}
