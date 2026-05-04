use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const SOKOBAN_DIFFICULTY_EASY: &str = "easy";
pub const SOKOBAN_DIFFICULTY_NORMAL: &str = "normal";
pub const SOKOBAN_DIFFICULTY_HARD: &str = "hard";
pub const SOKOBAN_DIFFICULTY_ORDER: &[&str] = &[
    SOKOBAN_DIFFICULTY_EASY,
    SOKOBAN_DIFFICULTY_NORMAL,
    SOKOBAN_DIFFICULTY_HARD,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SokobanDifficultyConfig {
    #[serde(default)]
    pub daily_plays: i32,
    #[serde(default)]
    pub level_count: i32,
    #[serde(default)]
    pub reward_amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SokobanConfigResponse {
    #[serde(default)]
    pub difficulties: HashMap<String, SokobanDifficultyConfig>,
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
pub struct SokobanSession {
    #[serde(default)]
    pub session_id: i32,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub level_index: i32,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub won: bool,
    #[serde(default)]
    pub player: [i32; 2],
    #[serde(default)]
    pub starting_player: [i32; 2],
    #[serde(default)]
    pub boxes: Vec<[i32; 2]>,
    #[serde(default)]
    pub starting_boxes: Vec<[i32; 2]>,
    #[serde(default)]
    pub targets: Vec<[i32; 2]>,
    #[serde(default)]
    pub walls: Vec<[i32; 2]>,
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub move_count: i32,
    #[serde(default)]
    pub push_count: i32,
    #[serde(default)]
    pub reward_amount: f64,
    #[serde(default)]
    pub server_seed_hash: String,
    #[serde(default)]
    pub schema_version: i32,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub started_at_ms: i64,
    #[serde(default)]
    pub ended_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SokobanMeResponse {
    #[serde(default)]
    pub active_session: Option<SokobanSession>,
    #[serde(default)]
    pub daily_plays_remaining: HashMap<String, i32>,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub server_now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SokobanStartRequest {
    pub difficulty: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SokobanStartResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub session: SokobanSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SokobanMoveRequest {
    pub session_id: i32,
    pub direction: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SokobanMoveResponse {
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub changed: bool,
    #[serde(default)]
    pub session: SokobanSession,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_response_accepts_api_payload() {
        let response: SokobanConfigResponse = serde_json::from_str(
            r#"{"difficulties":{"easy":{"daily_plays":5,"level_count":4,"reward_amount":0.3},"normal":{"daily_plays":4,"level_count":3,"reward_amount":1.0},"hard":{"daily_plays":3,"level_count":3,"reward_amount":3.0}},"directions":["up","down","left","right"],"max_active_sessions":1,"max_moves":1000,"min_interval_ms":40}"#,
        )
        .unwrap();
        assert_eq!(response.max_moves, 1000);
        assert_eq!(response.difficulties["hard"].reward_amount, 3.0);
    }

    #[test]
    fn me_and_session_responses_accept_api_payloads() {
        let me: SokobanMeResponse = serde_json::from_str(
            r#"{"active_session":null,"daily_plays_remaining":{"easy":5,"hard":3,"normal":4},"ok":true,"server_now_ms":1}"#,
        )
        .unwrap();
        assert_eq!(me.daily_plays_remaining["easy"], 5);

        let start: SokobanStartResponse = serde_json::from_str(
            r#"{"ok":true,"session":{"boxes":[[2,3]],"created_at":"now","difficulty":"easy","ended_at_ms":null,"height":6,"level_index":2,"move_count":0,"player":[3,4],"push_count":0,"reward_amount":0.0,"schema_version":1,"server_seed_hash":"abc","session_id":1,"started_at_ms":1,"starting_boxes":[[2,3]],"starting_player":[3,4],"status":"pending","targets":[[1,3]],"walls":[[0,0]],"width":9,"won":false}}"#,
        )
        .unwrap();
        assert!(start.ok);
        assert_eq!(start.session.session_id, 1);
    }
}
