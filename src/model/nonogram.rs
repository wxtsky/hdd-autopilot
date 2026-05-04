use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const NONOGRAM_DIFFICULTY_EASY: &str = "easy";
pub const NONOGRAM_DIFFICULTY_NORMAL: &str = "normal";
pub const NONOGRAM_DIFFICULTY_HARD: &str = "hard";
pub const NONOGRAM_DIFFICULTY_ORDER: &[&str] = &[
    NONOGRAM_DIFFICULTY_EASY,
    NONOGRAM_DIFFICULTY_NORMAL,
    NONOGRAM_DIFFICULTY_HARD,
];

pub const NONOGRAM_ACTION_FILL: &str = "fill";
pub const NONOGRAM_ACTION_CROSS: &str = "cross";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NonogramDifficultyConfig {
    #[serde(default)]
    pub daily_plays: i32,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub density: f64,
    #[serde(default)]
    pub reward_amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NonogramConfigResponse {
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub cell_states: HashMap<String, i32>,
    #[serde(default)]
    pub difficulties: HashMap<String, NonogramDifficultyConfig>,
    #[serde(default)]
    pub max_active_sessions: i32,
    #[serde(default)]
    pub max_moves: i32,
    #[serde(default)]
    pub min_interval_ms: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NonogramSession {
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
    /// 0=空，1=填，2=叉
    #[serde(default)]
    pub cells: Vec<Vec<i32>>,
    #[serde(default)]
    pub row_clues: Vec<Vec<i32>>,
    #[serde(default)]
    pub col_clues: Vec<Vec<i32>>,
    #[serde(default)]
    pub click_count: i32,
    #[serde(default)]
    pub reward_amount: f64,
    #[serde(default)]
    pub schema_version: i32,
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
pub struct NonogramMeResponse {
    #[serde(default)]
    pub active_session: Option<NonogramSession>,
    #[serde(default)]
    pub daily_plays_remaining: HashMap<String, i32>,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub server_now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NonogramStartRequest {
    pub difficulty: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NonogramStartResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub session: NonogramSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct NonogramClickRequest {
    pub session_id: i32,
    pub action: String,
    pub r: i32,
    pub c: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NonogramClickResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub session: NonogramSession,
}
