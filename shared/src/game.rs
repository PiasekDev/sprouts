use serde::{Deserialize, Serialize};
use serde_fields::SerdeField;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardState {
	pub spots: Vec<Spot>,
	pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spot {
	pub id: i32,
	pub x: f64,
	pub y: f64,
	pub degree: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
	pub id: i32,
	pub start_spot_id: i32,
	pub end_spot_id: i32,
	pub points: Vec<[f64; 2]>,
	pub new_spot_id: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSpot {
	pub x: f64,
	pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, SerdeField)]
pub struct MoveRequest {
	pub start_spot_id: i32,
	pub end_spot_id: i32,
	pub points: Vec<[f64; 2]>,
	pub new_spot: NewSpot,
}

#[derive(Debug, Clone, Serialize, Deserialize, SerdeField)]
pub struct JoinGameRequest {
	pub join_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GamePlayer {
	pub id: Uuid,
	pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GameResponse {
	Waiting {
		id: Uuid,
		join_code: String,
		player1: GamePlayer,
		board_state: BoardState,
	},
	Active {
		id: Uuid,
		join_code: String,
		player1: GamePlayer,
		player2: GamePlayer,
		current_turn_user_id: Uuid,
		board_state: BoardState,
	},
	Finished {
		id: Uuid,
		join_code: String,
		player1: GamePlayer,
		player2: GamePlayer,
		winner_user_id: Uuid,
		board_state: BoardState,
	},
}

impl BoardState {
	pub fn initial() -> Self {
		Self {
			spots: vec![
				Spot {
					id: 1,
					x: 100.0,
					y: 100.0,
					degree: 0,
				},
				Spot {
					id: 2,
					x: 300.0,
					y: 100.0,
					degree: 0,
				},
			],
			edges: Vec::new(),
		}
	}

	pub fn find_spot(&self, spot_id: i32) -> Option<&Spot> {
		self.spots.iter().find(|spot| spot.id == spot_id)
	}

	pub fn next_spot_id(&self) -> i32 {
		self.spots.iter().map(|spot| spot.id).max().unwrap_or(0) + 1
	}

	pub fn next_edge_id(&self) -> i32 {
		self.edges.iter().map(|edge| edge.id).max().unwrap_or(0) + 1
	}
}

impl GameResponse {
	pub fn id(&self) -> Uuid {
		match self {
			Self::Waiting { id, .. } | Self::Active { id, .. } | Self::Finished { id, .. } => *id,
		}
	}

	pub fn join_code(&self) -> &str {
		match self {
			Self::Waiting { join_code, .. }
			| Self::Active { join_code, .. }
			| Self::Finished { join_code, .. } => join_code,
		}
	}

	pub fn board_state(&self) -> &BoardState {
		match self {
			Self::Waiting { board_state, .. }
			| Self::Active { board_state, .. }
			| Self::Finished { board_state, .. } => board_state,
		}
	}
}
