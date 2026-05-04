use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::model::{AuthCache, AuthConfig, SOKOBAN_DIFFICULTY_ORDER, SokobanSession};
use crate::runtime::resolve_data_file_path;
use crate::ui;
use crate::workflows::common::{
    AccountRewardSummary, AccountRuntime, BatchState, ensure_authenticated, format_amount,
    print_account_reward_summary, run_account_task_until_complete,
    with_auth_retry_api_until_success,
};

pub const DONE_MESSAGE: &str = "自动推箱子已完成。";

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
        result_log_dir: resolve_data_file_path("log/sokoban"),
        log: log.clone(),
    }));
    let accounts = config.accounts.clone();
    let base_url = config.base_url.clone();
    log.line_fmt(format_args!(
        "开始自动推箱子，本次会处理 {} 个账号。",
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
                    "自动推箱子",
                    &email,
                    || run_account(&cancel_flag, &state, &mut runtime),
                )?;
                let total_reward = summaries.iter().map(|s| s.total_reward).sum();
                Ok(AccountRewardSummary {
                    index,
                    email: summaries
                        .first()
                        .map(|s| s.email.clone())
                        .unwrap_or(email),
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
                .line("自动推箱子任务异常退出，请查看前面的账号日志定位原因。"),
        }
    }
    print_account_reward_summary(log, "自动推箱子", &reward_summaries);
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
        result_log_dir: resolve_data_file_path("log/sokoban"),
        log: log.clone(),
    }));
    let mut runtime = AccountRuntime::new(&config.base_url, account);
    let task_log = state.lock().unwrap().log.clone();
    let email = runtime.email().to_string();
    let summaries = run_account_task_until_complete(
        cancel_flag,
        &task_log,
        "自动推箱子",
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
    failed: i32,
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
        "sokoban config",
        |client, auth_token| client.get_sokoban_config(auth_token),
    )?;
    let interval = Duration::from_millis(config.min_interval_ms.max(40) as u64);
    state.lock().unwrap().log.line_fmt(format_args!(
        "账号 {} 已准备好：推箱子包含 {} 难度，最小操作间隔 {}ms。",
        runtime.email(),
        SOKOBAN_DIFFICULTY_ORDER.join(" / "),
        config.min_interval_ms,
    ));

    // 处理残局
    let me = with_auth_retry_api_until_success(
        cancel_flag,
        state,
        runtime,
        "sokoban me",
        |client, auth_token| client.get_sokoban_me(auth_token),
    )?;
    let mut summaries: Vec<DifficultySummary> = Vec::new();
    let mut total_reward = 0.0_f64;

    if let Some(active) = me.active_session.clone() {
        if active.status == "pending" || active.status.is_empty() {
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 发现推箱子{}残局（对局 {}），先继续玩完。",
                runtime.email(),
                active.difficulty,
                active.session_id,
            ));
            let result = play_until_done(cancel_flag, state, runtime, active, interval)?;
            total_reward += result.reward;
            let summary = bump(&mut summaries, runtime.email(), &result);
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 的推箱子{}残局结束：{}（奖励 {}）",
                runtime.email(),
                summary.difficulty,
                if result.won { "成功" } else { "失败" },
                format_amount(result.reward),
            ));
        }
    }

    for difficulty in SOKOBAN_DIFFICULTY_ORDER.iter() {
        ui::check_cancel(cancel_flag)?;
        let me = with_auth_retry_api_until_success(
            cancel_flag,
            state,
            runtime,
            "sokoban me",
            |client, auth_token| client.get_sokoban_me(auth_token),
        )?;
        let remaining = me
            .daily_plays_remaining
            .get(*difficulty)
            .copied()
            .unwrap_or(0);
        let reward_per_play = config
            .difficulties
            .get(*difficulty)
            .map(|c| c.reward_amount)
            .unwrap_or(0.0);
        if remaining <= 0 {
            ensure_summary(&mut summaries, runtime.email(), difficulty);
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 的推箱子{}难度今天已经领取完了。",
                runtime.email(),
                difficulty,
            ));
            continue;
        }
        state.lock().unwrap().log.line_fmt(format_args!(
            "账号 {} 推箱子{}难度今天剩 {} 局（每局奖励 {}）。",
            runtime.email(),
            difficulty,
            remaining,
            format_amount(reward_per_play),
        ));
        for play_index in 0..remaining {
            ui::check_cancel(cancel_flag)?;
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 开始玩推箱子{}难度，今天第 {}/{} 局。",
                runtime.email(),
                difficulty,
                play_index + 1,
                remaining,
            ));
            let start = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "sokoban start",
                |client, auth_token| client.start_sokoban(auth_token, difficulty),
            )?;
            let result = play_until_done(cancel_flag, state, runtime, start.session, interval)?;
            total_reward += result.reward;
            bump(&mut summaries, runtime.email(), &result);
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 推箱子{}第 {}/{} 局：{}（奖励 {}）",
                runtime.email(),
                difficulty,
                play_index + 1,
                remaining,
                if result.won { "成功" } else { "失败" },
                format_amount(result.reward),
            ));
        }
    }

    state.lock().unwrap().log.line_fmt(format_args!(
        "账号 {} 的自动推箱子运行完成，本次累计奖励 {}。",
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

fn play_until_done(
    cancel_flag: &ui::CancelFlag,
    state: &Arc<Mutex<BatchState>>,
    runtime: &mut AccountRuntime,
    initial: SokobanSession,
    interval: Duration,
) -> io::Result<RoundResult> {
    let mut session = initial;
    let mut total_steps = 0_usize;
    loop {
        if session.status == "won" || session.won {
            return Ok(RoundResult {
                difficulty: session.difficulty.clone(),
                won: true,
                reward: session.reward_amount,
            });
        }
        if matches!(session.status.as_str(), "failed" | "abandoned" | "lost") {
            return Ok(RoundResult {
                difficulty: session.difficulty.clone(),
                won: false,
                reward: 0.0,
            });
        }
        ui::check_cancel(cancel_flag)?;
        let path = match crate::solver::sokoban::solve(&session) {
            Some(p) => p,
            None => {
                state.lock().unwrap().log.line_fmt(format_args!(
                    "账号 {} 推箱子求解失败：对局 {} 暂时无解。",
                    runtime.email(),
                    session.session_id,
                ));
                return Ok(RoundResult {
                    difficulty: session.difficulty.clone(),
                    won: false,
                    reward: 0.0,
                });
            }
        };
        if path.is_empty() {
            // 已完成但 status 还没翻 — 拉一次最新 session
            let me = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "sokoban me",
                |client, auth_token| client.get_sokoban_me(auth_token),
            )?;
            if let Some(active) = me.active_session {
                session = active;
                continue;
            }
            return Ok(RoundResult {
                difficulty: session.difficulty.clone(),
                won: true,
                reward: session.reward_amount,
            });
        }
        let session_id = session.session_id;
        for direction in path {
            ui::check_cancel(cancel_flag)?;
            ui::sleep_with_cancel(cancel_flag, interval)?;
            let resp = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "sokoban move",
                |client, auth_token| client.move_sokoban(auth_token, session_id, direction),
            )?;
            session = resp.session;
            total_steps += 1;
            if session.status == "won"
                || session.won
                || matches!(session.status.as_str(), "failed" | "abandoned" | "lost")
            {
                break;
            }
            if total_steps > 10_000 {
                return Ok(RoundResult {
                    difficulty: session.difficulty.clone(),
                    won: false,
                    reward: 0.0,
                });
            }
        }
    }
}

fn ensure_summary(list: &mut Vec<DifficultySummary>, email: &str, difficulty: &str) {
    if !list.iter().any(|s| s.difficulty == difficulty) {
        list.push(DifficultySummary {
            email: email.to_string(),
            difficulty: difficulty.to_string(),
            ..DifficultySummary::default()
        });
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
        entry.failed += 1;
    }
    entry.total_reward += result.reward;
    entry.clone()
}
