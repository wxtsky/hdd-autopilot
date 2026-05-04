use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const MAZE_DIFFICULTY_EASY: &str = "easy";
pub const MAZE_DIFFICULTY_NORMAL: &str = "normal";
pub const MAZE_DIFFICULTY_HARD: &str = "hard";
pub const MAZE_DIFFICULTY_ORDER: &[&str] = &[
    MAZE_DIFFICULTY_EASY,
    MAZE_DIFFICULTY_NORMAL,
    MAZE_DIFFICULTY_HARD,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MazeDifficultyConfig {
    #[serde(default)]
    pub daily_plays: i32,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub reward_amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MazeConfigResponse {
    #[serde(default)]
    pub difficulties: HashMap<String, MazeDifficultyConfig>,
    #[serde(default)]
    pub directions: Vec<String>,
    #[serde(default)]
    pub max_active_sessions: i32,
    #[serde(default)]
    pub max_moves: i32,
    #[serde(default)]
    pub min_interval_ms: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MazeSession {
    #[serde(default)]
    pub session_id: i32,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub won: bool,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub player: [i32; 2],
    #[serde(default)]
    pub exit: [i32; 2],
    /// `open_edges`: 每条边是 [[r1, c1], [r2, c2]]，表示这两个相邻格之间是开的（无墙）。
    #[serde(default)]
    pub open_edges: Vec<[[i32; 2]; 2]>,
    #[serde(default)]
    pub move_count: i32,
    #[serde(default)]
    pub reward_amount: f64,
    #[serde(default)]
    pub server_seed_hash: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub started_at_ms: i64,
    #[serde(default)]
    pub ended_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MazeMeResponse {
    #[serde(default)]
    pub active_session: Option<MazeSession>,
    #[serde(default)]
    pub daily_plays_remaining: HashMap<String, i32>,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub server_now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MazeStartRequest {
    pub difficulty: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MazeStartResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub session: MazeSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MazeMoveRequest {
    pub session_id: i32,
    pub direction: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MazeMoveResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub changed: bool,
    #[serde(default)]
    pub session: MazeSession,
}
