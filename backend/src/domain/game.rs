use serde::{Deserialize, Serialize};
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
pub struct SubmittedMove {
	pub start_spot_id: i32,
	pub end_spot_id: i32,
	pub points: Vec<[f64; 2]>,
	pub new_spot: NewSpot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSpot {
	pub x: f64,
	pub y: f64,
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
