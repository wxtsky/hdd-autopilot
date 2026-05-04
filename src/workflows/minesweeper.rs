use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::model::{
    AuthCache, AuthConfig, MINESWEEPER_DIFFICULTY_ORDER, MinesweeperClickResponse,
    MinesweeperConfigResponse, MinesweeperPlayState,
};
use crate::runtime::resolve_data_file_path;
use crate::solver::minesweeper::{Action as MineAction, Board, next_actions};
use crate::ui;
use crate::workflows::common::{
    AccountRewardSummary, AccountRuntime, BatchState, ensure_authenticated, format_amount,
    print_account_reward_summary, run_account_task_until_complete,
    with_auth_retry_api_until_success,
};

pub const DONE_MESSAGE: &str = "自动扫雷已完成。";

#[derive(Debug, Clone)]
pub struct AccountRunOutput {
    pub account: AuthCache,
    pub total_reward: f64,
}

pub fn run_batch(
    config: AuthConfig,
    auth_cache_file: impl AsRef<Path>,
    cancel_flag: &ui::CancelFlag,
    log: &ui::TaskLog,
) -> io::Result<AuthConfig> {
    if config.accounts.is_empty() {
        println!("当前还没有可用账号。");
        return Ok(config);
    }
    let state = Arc::new(Mutex::new(BatchState {
        config: config.clone(),
        auth_cache_file: Some(auth_cache_file.as_ref().to_path_buf()),
        result_log_dir: resolve_data_file_path("log/minesweeper"),
        log: log.clone(),
    }));
    let accounts = config.accounts.clone();
    let base_url = config.base_url.clone();
    log.line_fmt(format_args!(
        "开始自动扫雷，本次会处理 {} 个账号。",
        accounts.len()
    ));
    let mut reward_summaries = accounts
        .iter()
        .enumerate()
        .map(|(index, account)| AccountRewardSummary {
            index,
            email: account.email.trim().to_string(),
            total_reward: 0.0,
        })
        .collect::<Vec<_>>();
    let mut handles = Vec::with_capacity(accounts.len());
    for (index, account) in accounts.into_iter().enumerate() {
        ui::check_cancel(cancel_flag)?;
        let state = Arc::clone(&state);
        let cancel_flag = Arc::clone(cancel_flag);
        let base_url = base_url.clone();
        handles.push(thread::spawn(
            move || -> io::Result<AccountRewardSummary> {
                let mut runtime = AccountRuntime::new(&base_url, account);
                let email = runtime.email().to_string();
                let task_log = state.lock().unwrap().log.clone();
                let summaries = run_account_task_until_complete(
                    &cancel_flag,
                    &task_log,
                    "自动扫雷",
                    &email,
                    || run_account(&cancel_flag, &state, &mut runtime),
                )?;
                let total_reward = summaries.iter().map(|s| s.total_reward).sum();
                Ok(AccountRewardSummary {
                    index,
                    email: summaries.first().map(|s| s.email.clone()).unwrap_or(email),
                    total_reward,
                })
            },
        ));
    }
    for handle in handles {
        match handle.join() {
            Ok(Ok(summary)) => {
                if let Some(slot) = reward_summaries.get_mut(summary.index) {
                    *slot = summary;
                }
            }
            Ok(Err(error)) if error.kind() == io::ErrorKind::Interrupted => return Err(error),
            Ok(Err(error)) => return Err(error),
            Err(_) => state
                .lock()
                .unwrap()
                .log
                .line("自动扫雷任务异常退出，请查看前面的账号日志定位原因。"),
        }
    }
    print_account_reward_summary(log, "自动扫雷", &reward_summaries);
    Ok(state.lock().unwrap().config.clone())
}

pub fn run_account_for_free_play_with_log(
    config: &AuthConfig,
    account: AuthCache,
    cancel_flag: &ui::CancelFlag,
    log: &ui::TaskLog,
) -> io::Result<AccountRunOutput> {
    let fallback_account = account.clone();
    let state = Arc::new(Mutex::new(BatchState {
        config: AuthConfig {
            base_url: config.base_url.clone(),
            accounts: vec![account.clone()],
        },
        auth_cache_file: None,
        result_log_dir: resolve_data_file_path("log/minesweeper"),
        log: log.clone(),
    }));
    let mut runtime = AccountRuntime::new(&config.base_url, account);
    let task_log = state.lock().unwrap().log.clone();
    let email = runtime.email().to_string();
    let summaries = run_account_task_until_complete(
        cancel_flag,
        &task_log,
        "自动扫雷",
        &email,
        || run_account(cancel_flag, &state, &mut runtime),
    )?;
    let total_reward = summaries.iter().map(|s| s.total_reward).sum();
    let updated_account = state
        .lock()
        .unwrap()
        .config
        .accounts
        .first()
        .cloned()
        .unwrap_or(fallback_account);
    Ok(AccountRunOutput {
        account: updated_account,
        total_reward,
    })
}

#[derive(Debug, Clone, Default)]
struct DifficultySummary {
    email: String,
    difficulty: String,
    played: i32,
    won: i32,
    lost: i32,
    total_reward: f64,
}

fn run_account(
    cancel_flag: &ui::CancelFlag,
    state: &Arc<Mutex<BatchState>>,
    runtime: &mut AccountRuntime,
) -> io::Result<Vec<DifficultySummary>> {
    ui::check_cancel(cancel_flag)?;
    ensure_authenticated(state, runtime)?;

    let config = with_auth_retry_api_until_success(
        cancel_flag,
        state,
        runtime,
        "minesweeper config",
        |client, auth_token| client.get_minesweeper_config(auth_token),
    )?;
    let interval = Duration::from_millis(config.min_interval_ms.max(60) as u64);
    state.lock().unwrap().log.line_fmt(format_args!(
        "账号 {} 已准备好：扫雷包含 {} 难度，最小操作间隔 {}ms。",
        runtime.email(),
        MINESWEEPER_DIFFICULTY_ORDER.join(" / "),
        config.min_interval_ms,
    ));

    // 处理残局：如果有 active_round 且 resolution=pending，要么继续要么 abandon
    let me = with_auth_retry_api_until_success(
        cancel_flag,
        state,
        runtime,
        "minesweeper me",
        |client, auth_token| client.get_minesweeper_me(auth_token),
    )?;
    let mut summaries: Vec<DifficultySummary> = Vec::new();
    let mut total_reward = 0.0_f64;
    let mut play_count_today: usize = 0;

    if let Some(active) = me.active_round.clone() {
        if active.resolution == "pending" || active.resolution.is_empty() {
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 发现扫雷{}残局（对局 {}），先放弃以释放槽位。",
                runtime.email(),
                active.difficulty,
                active.play_id,
            ));
            // 简化：直接 abandon 残局，避免本地状态不一致
            let _ = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "minesweeper abandon",
                |client, auth_token| client.abandon_minesweeper(auth_token, active.play_id),
            );
        }
    }

    for difficulty in MINESWEEPER_DIFFICULTY_ORDER.iter() {
        ui::check_cancel(cancel_flag)?;
        let reward_per_play = config
            .rewards
            .get(*difficulty)
            .copied()
            .unwrap_or(0.0);
        let dcfg = match config.difficulties.get(*difficulty) {
            Some(c) => c,
            None => continue,
        };
        // 服务端只给总配额（max_plays_per_day）；按难度尝试一次（每难度第一次 won 才领奖）。
        // 实际策略：每个难度尝试到第一次成功，最多 5 次（避免无限尝试）。
        let mut won_this_difficulty = false;
        let max_attempts = 5;
        for attempt in 0..max_attempts {
            ui::check_cancel(cancel_flag)?;
            if play_count_today >= config.max_plays_per_day as usize {
                break;
            }
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 开始扫雷{}难度第 {} 次尝试（每局奖励 {}）。",
                runtime.email(),
                difficulty,
                attempt + 1,
                format_amount(reward_per_play),
            ));
            let start = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "minesweeper start",
                |client, auth_token| client.start_minesweeper(auth_token, difficulty),
            )?;
            play_count_today += 1;
            let result = play_one_round(cancel_flag, state, runtime, start, dcfg.rows, dcfg.cols, dcfg.mines, interval)?;
            total_reward += result.reward;
            bump(&mut summaries, runtime.email(), &result);
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 扫雷{}难度第 {} 次：{}（奖励 {}）",
                runtime.email(),
                difficulty,
                attempt + 1,
                if result.won { "通关" } else { "踩雷" },
                format_amount(result.reward),
            ));
            if result.won {
                won_this_difficulty = true;
                break;
            }
        }
        if !won_this_difficulty {
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 扫雷{}难度 {} 次都没通关，跳过。",
                runtime.email(),
                difficulty,
                max_attempts,
            ));
        }
    }

    state.lock().unwrap().log.line_fmt(format_args!(
        "账号 {} 的自动扫雷运行完成，本次累计奖励 {}。",
        runtime.email(),
        format_amount(total_reward),
    ));
    Ok(summaries)
}

struct RoundResult {
    difficulty: String,
    won: bool,
    reward: f64,
}

fn play_one_round(
    cancel_flag: &ui::CancelFlag,
    state: &Arc<Mutex<BatchState>>,
    runtime: &mut AccountRuntime,
    start: MinesweeperPlayState,
    rows: i32,
    cols: i32,
    mines: i32,
    interval: Duration,
) -> io::Result<RoundResult> {
    let play_id = start.play_id;
    let difficulty = start.difficulty.clone();
    let mut board = Board::new(rows, cols, mines);
    // 应用 start 的 revealed_numbers（如果有）
    apply_revealed(&mut board, &start.revealed_numbers);
    apply_flagged_grid(&mut board, &start.flagged);

    // 第一次点击：若 board 完全 unknown 则点中心
    let first_click = start.first_click;
    let mut clicks = 0;
    let max_clicks = (rows * cols * 4) as usize;
    let mut last_resolution = start.resolution.clone();
    let mut last_reward = start.reward_amount;

    if first_click.is_none() {
        let cx = cols / 2;
        let cy = rows / 2;
        let resp = match click(
            cancel_flag,
            state,
            runtime,
            play_id,
            "reveal",
            cx,
            cy,
            interval,
        ) {
            Ok(r) => r,
            Err(error) => {
                state.lock().unwrap().log.line_fmt(format_args!(
                    "账号 {} 扫雷{}首次点击失败：{}（这一局视为失败，继续后续）",
                    runtime.email(),
                    difficulty,
                    error
                ));
                let _ = with_auth_retry_api_until_success(
                    cancel_flag,
                    state,
                    runtime,
                    "minesweeper abandon",
                    |client, auth_token| client.abandon_minesweeper(auth_token, play_id),
                );
                return Ok(RoundResult {
                    difficulty,
                    won: false,
                    reward: 0.0,
                });
            }
        };
        clicks += 1;
        apply_click_response(&mut board, &resp);
        last_resolution = resp.state.resolution.clone();
        last_reward = resp.state.reward_amount;
        if resp.delta.lost {
            return Ok(RoundResult {
                difficulty,
                won: false,
                reward: 0.0,
            });
        }
    }

    while last_resolution == "pending" || last_resolution.is_empty() {
        if clicks > max_clicks {
            // 防御性退出：abandon
            let _ = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "minesweeper abandon",
                |client, auth_token| client.abandon_minesweeper(auth_token, play_id),
            );
            return Ok(RoundResult {
                difficulty,
                won: false,
                reward: 0.0,
            });
        }
        let actions = next_actions(&board);
        if actions.is_empty() {
            // 没有可行动作 — 整局应该已经结束，但服务端没发完成？拉一次 me
            return Ok(RoundResult {
                difficulty,
                won: matches!(last_resolution.as_str(), "won"),
                reward: if last_resolution == "won" {
                    last_reward
                } else {
                    0.0
                },
            });
        }
        for action in actions {
            ui::check_cancel(cancel_flag)?;
            // flag 仅本地标记，不发请求（服务端胜利判定只看 reveal）
            let (x, y) = match action {
                MineAction::Flag { x, y } => {
                    board.set_flag(x, y);
                    continue;
                }
                MineAction::Reveal { x, y } => (x, y),
            };
            let resp = match click(
                cancel_flag,
                state,
                runtime,
                play_id,
                "reveal",
                x,
                y,
                interval,
            ) {
                Ok(r) => r,
                Err(error) => {
                    state.lock().unwrap().log.line_fmt(format_args!(
                        "账号 {} 扫雷{}操作 reveal({}, {}) 失败：{}（这一局视为失败，放弃继续）",
                        runtime.email(),
                        difficulty,
                        x,
                        y,
                        error
                    ));
                    let _ = with_auth_retry_api_until_success(
                        cancel_flag,
                        state,
                        runtime,
                        "minesweeper abandon",
                        |client, auth_token| client.abandon_minesweeper(auth_token, play_id),
                    );
                    return Ok(RoundResult {
                        difficulty,
                        won: false,
                        reward: 0.0,
                    });
                }
            };
            clicks += 1;
            apply_click_response(&mut board, &resp);
            last_resolution = resp.state.resolution.clone();
            last_reward = resp.state.reward_amount;
            if resp.delta.lost {
                return Ok(RoundResult {
                    difficulty,
                    won: false,
                    reward: 0.0,
                });
            }
            if last_resolution == "won" {
                return Ok(RoundResult {
                    difficulty,
                    won: true,
                    reward: last_reward,
                });
            }
            if last_resolution == "lost" {
                return Ok(RoundResult {
                    difficulty,
                    won: false,
                    reward: 0.0,
                });
            }
        }
    }
    Ok(RoundResult {
        difficulty,
        won: matches!(last_resolution.as_str(), "won"),
        reward: if last_resolution == "won" {
            last_reward
        } else {
            0.0
        },
    })
}

fn click(
    cancel_flag: &ui::CancelFlag,
    state: &Arc<Mutex<BatchState>>,
    runtime: &mut AccountRuntime,
    play_id: i32,
    action: &str,
    x: i32,
    y: i32,
    interval: Duration,
) -> io::Result<MinesweeperClickResponse> {
    ui::sleep_with_cancel(cancel_flag, interval)?;
    with_auth_retry_api_until_success(
        cancel_flag,
        state,
        runtime,
        "minesweeper click",
        |client, auth_token| client.click_minesweeper(auth_token, play_id, action, x, y),
    )
}

fn apply_click_response(board: &mut Board, resp: &MinesweeperClickResponse) {
    apply_revealed(board, &resp.delta.revealed_cells);
    for cell in &resp.delta.flagged_cells {
        // MinesweeperFlagDelta(x, y, flagged)
        let x = cell.0;
        let y = cell.1;
        if cell.2 {
            board.set_flag(x, y);
        } else {
            board.set_unknown(x, y);
        }
    }
    // 整体 revealed_numbers 也尝试用一下（兜底）
    apply_revealed(board, &resp.state.revealed_numbers);
}

fn apply_revealed(board: &mut Board, cells: &[[i32; 3]]) {
    for cell in cells {
        let x = cell[0];
        let y = cell[1];
        let n = cell[2].max(0).min(8) as u8;
        board.set_number(x, y, n);
    }
}

fn apply_flagged_grid(board: &mut Board, flagged: &[Vec<bool>]) {
    for (y, row) in flagged.iter().enumerate() {
        for (x, &is_flag) in row.iter().enumerate() {
            if is_flag {
                board.set_flag(x as i32, y as i32);
            }
        }
    }
}

fn bump(list: &mut Vec<DifficultySummary>, email: &str, result: &RoundResult) -> DifficultySummary {
    let pos = list.iter().position(|s| s.difficulty == result.difficulty);
    let idx = match pos {
        Some(i) => i,
        None => {
            list.push(DifficultySummary {
                email: email.to_string(),
                difficulty: result.difficulty.clone(),
                ..DifficultySummary::default()
            });
            list.len() - 1
        }
    };
    let entry = &mut list[idx];
    entry.played += 1;
    if result.won {
        entry.won += 1;
    } else {
        entry.lost += 1;
    }
    entry.total_reward += result.reward;
    entry.clone()
}

fn _unused(_: &MinesweeperConfigResponse) {}
