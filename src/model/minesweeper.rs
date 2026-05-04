use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub const MINESWEEPER_DIFFICULTY_BEGINNER: &str = "beginner";
pub const MINESWEEPER_DIFFICULTY_INTERMEDIATE: &str = "intermediate";
pub const MINESWEEPER_DIFFICULTY_EXPERT: &str = "expert";
pub const MINESWEEPER_DIFFICULTY_ORDER: &[&str] = &[
    MINESWEEPER_DIFFICULTY_BEGINNER,
    MINESWEEPER_DIFFICULTY_INTERMEDIATE,
    MINESWEEPER_DIFFICULTY_EXPERT,
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinesweeperDifficultyConfig {
    #[serde(default)]
    pub cols: i32,
    #[serde(default)]
    pub rows: i32,
    #[serde(default)]
    pub mines: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinesweeperConfigResponse {
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub difficulties: HashMap<String, MinesweeperDifficultyConfig>,
    #[serde(default)]
    pub max_plays_per_day: i32,
    #[serde(default)]
    pub min_interval_ms: i32,
    #[serde(default)]
    pub minesweeper_hmac_prefix: String,
    #[serde(default)]
    pub rewards: HashMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinesweeperPlayState {
    #[serde(default)]
    pub play_id: i32,
    #[serde(default)]
    pub difficulty: String,
    #[serde(default)]
    pub rows: i32,
    #[serde(default)]
    pub cols: i32,
    #[serde(default)]
    pub mine_count: i32,
    #[serde(default)]
    pub mines: Vec<[i32; 2]>,
    #[serde(default)]
    pub revealed: Vec<Vec<bool>>,
    #[serde(default)]
    pub flagged: Vec<Vec<bool>>,
    #[serde(default)]
    pub revealed_numbers: Vec<[i32; 3]>,
    #[serde(default)]
    pub first_click: Option<[i32; 2]>,
    #[serde(default)]
    pub resolution: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub reward_amount: f64,
    #[serde(default)]
    pub trace_count: i32,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub started_at_ms: Option<i64>,
    #[serde(default)]
    pub ended_at_ms: Option<i64>,
    #[serde(default)]
    pub server_seed: Option<String>,
    #[serde(default)]
    pub server_seed_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinesweeperMeResponse {
    #[serde(default)]
    pub active_round: Option<MinesweeperPlayState>,
    #[serde(default)]
    pub ok: bool,
    #[serde(default)]
    pub server_now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MinesweeperStartRequest {
    pub difficulty: String,
}

pub type MinesweeperStartResponse = MinesweeperPlayState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MinesweeperClickRequest {
    pub play_id: i32,
    pub action: String,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinesweeperFlagDelta(pub i32, pub i32, pub bool);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinesweeperClickDelta {
    #[serde(default)]
    pub first_click: Option<[i32; 2]>,
    #[serde(default)]
    pub revealed_cells: Vec<[i32; 3]>,
    #[serde(default)]
    pub flagged_cells: Vec<MinesweeperFlagDelta>,
    #[serde(default)]
    pub hit_mine: bool,
    #[serde(default)]
    pub lost: bool,
    #[serde(default)]
    pub won: bool,
}

/// 服务端的 click 接口返回的就是完整的 play state（带 delta 字段）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MinesweeperClickResponse {
    #[serde(flatten)]
    pub state: MinesweeperPlayState,
    #[serde(default)]
    pub delta: MinesweeperClickDelta,
    #[serde(default)]
    pub balance: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_response_accepts_api_payload() {
        let r: MinesweeperConfigResponse = serde_json::from_str(
            r#"{"actions":["reveal","flag","unflag","chord"],"difficulties":{"beginner":{"cols":8,"rows":8,"mines":10},"intermediate":{"cols":16,"rows":16,"mines":40},"expert":{"cols":30,"rows":16,"mines":99}},"max_plays_per_day":100,"min_interval_ms":50,"minesweeper_hmac_prefix":"minesweeper:v1:","rewards":{"beginner":0.5,"intermediate":2.0,"expert":5.0}}"#,
        )
        .unwrap();
        assert_eq!(r.rewards["expert"], 5.0);
    }

    #[test]
    fn click_response_accepts_api_payload() {
        let r: MinesweeperClickResponse = serde_json::from_str(
            r#"{"cols":8,"rows":8,"play_id":1,"resolution":"pending","status":"active","mine_count":10,"flagged":[],"revealed":[],"revealed_numbers":[],"mines":[],"server_seed_hash":"h","first_click":[4,4],"delta":{"first_click":[4,4],"revealed_cells":[[4,4,0]],"flagged_cells":[],"hit_mine":false,"lost":false}}"#,
        )
        .unwrap();
        assert_eq!(r.state.play_id, 1);
        assert_eq!(r.delta.revealed_cells.len(), 1);
    }
}
