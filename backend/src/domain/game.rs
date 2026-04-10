use serde::{Deserialize, Serialize};
pub use shared::game::{BoardState, Edge, NewSpot, Spot};
use sqlx::Type;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "game_status", rename_all = "snake_case")]
pub enum GameStatus {
	Waiting,
	Active,
	Finished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmittedMove {
	pub start_spot_id: i32,
	pub end_spot_id: i32,
	pub points: Vec<[f64; 2]>,
	pub new_spot: NewSpot,
}
