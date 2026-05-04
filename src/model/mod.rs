pub(crate) fn is_zero_i32(value: &i32) -> bool {
    *value == 0
}

pub(crate) fn is_false(value: &bool) -> bool {
    !*value
}

mod auth;
mod checkin;
mod lightsout;
mod maze;
mod memory;
mod minesweeper;
mod nonogram;
mod puzzle_15;
mod puzzle_2048;
mod scratch;
mod sheepmatch;
mod sokoban;
mod sudoku;

pub use auth::{
    AuthCache, AuthConfig, AuthMeData, AuthMeResponse, AuthSession, LoginRequest, LoginResponse,
    LoginResponseData, LoginUser, SessionCookie,
};
pub use checkin::{
    CheckinClaimResponse, CheckinMeResponse, CheckinResult, CheckinTodayResponse, CheckinUser,
};
pub use memory::{
    MEMORY_DIFFICULTY_EASY, MEMORY_DIFFICULTY_HARD, MEMORY_DIFFICULTY_HELL,
    MEMORY_DIFFICULTY_NORMAL, MEMORY_DIFFICULTY_ORDER, MemoryCard, MemoryConfigResponse,
    MemoryDifficultyConfig, MemoryFlipRequest, MemoryFlipResponse, MemoryHistoryResponse,
    MemoryMeResponse, MemorySession, MemoryStartRequest, MemoryStartResponse,
};
pub use puzzle_15::{
    PUZZLE_15_DIFFICULTY_CLASSIC, PUZZLE_15_DIFFICULTY_EASY, PUZZLE_15_DIFFICULTY_HARD,
    PUZZLE_15_DIFFICULTY_ORDER, Puzzle15ConfigResponse, Puzzle15DifficultyConfig,
    Puzzle15HistoryResponse, Puzzle15MeResponse, Puzzle15MoveRequest, Puzzle15MoveResponse,
    Puzzle15Session, Puzzle15StartRequest, Puzzle15StartResponse,
};
pub use puzzle_2048::{
    PUZZLE_2048_DIFFICULTY_CLASSIC, PUZZLE_2048_DIFFICULTY_JUMBO, PUZZLE_2048_DIFFICULTY_MINI,
    PUZZLE_2048_DIFFICULTY_ORDER, Puzzle2048AbandonRequest, Puzzle2048ConfigResponse,
    Puzzle2048DifficultyConfig, Puzzle2048HistoryItem, Puzzle2048HistoryResponse,
    Puzzle2048MeResponse, Puzzle2048MoveRequest, Puzzle2048MoveResponse, Puzzle2048SpawnedTile,
    Puzzle2048StartRequest, Puzzle2048StartResponse,
};
pub use scratch::{
    SCRATCH_GAME_TYPE_ICON_MATCH, SCRATCH_GAME_TYPE_LUCKY_NUMBERS, SCRATCH_GAME_TYPE_PROGRESS_RUN,
    SCRATCH_GAME_TYPE_THREE_KIND, SCRATCH_GAME_TYPE_TREASURE_CHEST, ScratchCell, ScratchCheckpoint,
    ScratchChest, ScratchHistoryItem, ScratchHistoryResponse, ScratchIconCell, ScratchNumber,
    ScratchPlayRequest, ScratchPlayResponse, ScratchRevealRequest, ScratchRevealResponse,
    ScratchRoundResult, ScratchTicketPayload, scratch_reveal_ready_at,
};
pub use minesweeper::{
    MINESWEEPER_DIFFICULTY_BEGINNER, MINESWEEPER_DIFFICULTY_EXPERT,
    MINESWEEPER_DIFFICULTY_INTERMEDIATE, MINESWEEPER_DIFFICULTY_ORDER,
    MinesweeperClickDelta, MinesweeperClickRequest, MinesweeperClickResponse,
    MinesweeperConfigResponse, MinesweeperDifficultyConfig, MinesweeperFlagDelta,
    MinesweeperMeResponse, MinesweeperPlayState, MinesweeperStartRequest,
    MinesweeperStartResponse,
};
pub use sheepmatch::{
    AbandonRequest, AbandonResponse, AccountRunSummary, ConfigResponse, DIFFICULTY_EASY,
    DIFFICULTY_HARD, DIFFICULTY_HELL, DIFFICULTY_NORMAL, DIFFICULTY_ORDER, GameDifficultyConfig,
    HistoryEntry, HistoryItem, HistoryResponse, Powerups, RoundResultSummary, SessionSnapshot,
    StartRequest, StartResponse, StepRequest, StepResponse, Tile, TileMeResponse, TileMeUser,
};
pub use sokoban::{
    SOKOBAN_DIFFICULTY_EASY, SOKOBAN_DIFFICULTY_HARD, SOKOBAN_DIFFICULTY_NORMAL,
    SOKOBAN_DIFFICULTY_ORDER, SokobanConfigResponse, SokobanDifficultyConfig, SokobanMeResponse,
    SokobanMoveRequest, SokobanMoveResponse, SokobanSession, SokobanStartRequest,
    SokobanStartResponse,
};
pub use maze::{
    MAZE_DIFFICULTY_EASY, MAZE_DIFFICULTY_HARD, MAZE_DIFFICULTY_NORMAL, MAZE_DIFFICULTY_ORDER,
    MazeConfigResponse, MazeDifficultyConfig, MazeMeResponse, MazeMoveRequest, MazeMoveResponse,
    MazeSession, MazeStartRequest, MazeStartResponse,
};
pub use lightsout::{
    LIGHTSOUT_DIFFICULTY_EASY, LIGHTSOUT_DIFFICULTY_HARD, LIGHTSOUT_DIFFICULTY_NORMAL,
    LIGHTSOUT_DIFFICULTY_ORDER, LightsoutClickRequest, LightsoutClickResponse,
    LightsoutConfigResponse, LightsoutDifficultyConfig, LightsoutMeResponse, LightsoutSession,
    LightsoutStartRequest, LightsoutStartResponse,
};
pub use nonogram::{
    NONOGRAM_ACTION_CROSS, NONOGRAM_ACTION_FILL, NONOGRAM_DIFFICULTY_EASY,
    NONOGRAM_DIFFICULTY_HARD, NONOGRAM_DIFFICULTY_NORMAL, NONOGRAM_DIFFICULTY_ORDER,
    NonogramClickRequest, NonogramClickResponse, NonogramConfigResponse,
    NonogramDifficultyConfig, NonogramMeResponse, NonogramSession, NonogramStartRequest,
    NonogramStartResponse,
};
pub use sudoku::{
    SUDOKU_DIFFICULTY_EASY, SUDOKU_DIFFICULTY_EXPERT, SUDOKU_DIFFICULTY_HARD,
    SUDOKU_DIFFICULTY_NORMAL, SUDOKU_DIFFICULTY_ORDER, SudokuConfigResponse,
    SudokuDifficultyConfig, SudokuFillRequest, SudokuFillResponse, SudokuHistoryResponse,
    SudokuMeResponse, SudokuSession, SudokuStartRequest, SudokuStartResponse,
};
