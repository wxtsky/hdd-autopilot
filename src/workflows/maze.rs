use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::model::{AuthCache, AuthConfig, MAZE_DIFFICULTY_ORDER, MazeSession};
use crate::runtime::resolve_data_file_path;
use crate::ui;
use crate::workflows::common::{
    AccountRewardSummary, AccountRuntime, BatchState, ensure_authenticated, format_amount,
    print_account_reward_summary, run_account_task_until_complete,
    with_auth_retry_api_until_success,
};

pub const DONE_MESSAGE: &str = "自动迷宫已完成。";

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
        result_log_dir: resolve_data_file_path("log/maze"),
        log: log.clone(),
    }));
    let accounts = config.accounts.clone();
    let base_url = config.base_url.clone();
    log.line_fmt(format_args!("开始自动迷宫，本次会处理 {} 个账号。", accounts.len()));
    let mut reward_summaries = accounts
        .iter()
        .enumerate()
        .map(|(index, a)| AccountRewardSummary {
            index,
            email: a.email.trim().to_string(),
            total_reward: 0.0,
        })
        .collect::<Vec<_>>();
    let mut handles = Vec::with_capacity(accounts.len());
    for (index, account) in accounts.into_iter().enumerate() {
        ui::check_cancel(cancel_flag)?;
        let state = Arc::clone(&state);
        let cancel_flag = Arc::clone(cancel_flag);
        let base_url = base_url.clone();
        handles.push(thread::spawn(move || -> io::Result<AccountRewardSummary> {
            let mut runtime = AccountRuntime::new(&base_url, account);
            let email = runtime.email().to_string();
            let task_log = state.lock().unwrap().log.clone();
            let total = run_account_task_until_complete(
                &cancel_flag,
                &task_log,
                "自动迷宫",
                &email,
                || run_account(&cancel_flag, &state, &mut runtime),
            )?;
            Ok(AccountRewardSummary {
                index,
                email,
                total_reward: total,
            })
        }));
    }
    for h in handles {
        match h.join() {
            Ok(Ok(s)) => {
                if let Some(slot) = reward_summaries.get_mut(s.index) {
                    *slot = s;
                }
            }
            Ok(Err(e)) if e.kind() == io::ErrorKind::Interrupted => return Err(e),
            Ok(Err(e)) => return Err(e),
            Err(_) => state.lock().unwrap().log.line("自动迷宫任务异常退出。"),
        }
    }
    print_account_reward_summary(log, "自动迷宫", &reward_summaries);
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
        result_log_dir: resolve_data_file_path("log/maze"),
        log: log.clone(),
    }));
    let mut runtime = AccountRuntime::new(&config.base_url, account);
    let task_log = state.lock().unwrap().log.clone();
    let email = runtime.email().to_string();
    let total = run_account_task_until_complete(
        cancel_flag,
        &task_log,
        "自动迷宫",
        &email,
        || run_account(cancel_flag, &state, &mut runtime),
    )?;
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
        total_reward: total,
    })
}

fn run_account(
    cancel_flag: &ui::CancelFlag,
    state: &Arc<Mutex<BatchState>>,
    runtime: &mut AccountRuntime,
) -> io::Result<f64> {
    ui::check_cancel(cancel_flag)?;
    ensure_authenticated(state, runtime)?;
    let config = with_auth_retry_api_until_success(
        cancel_flag,
        state,
        runtime,
        "maze config",
        |c, t| c.get_maze_config(t),
    )?;
    let interval = Duration::from_millis(config.min_interval_ms.max(30) as u64);
    state.lock().unwrap().log.line_fmt(format_args!(
        "账号 {} 已准备好：迷宫 3 档难度，最小操作间隔 {}ms。",
        runtime.email(),
        config.min_interval_ms
    ));

    // 处理残局
    let me = with_auth_retry_api_until_success(
        cancel_flag,
        state,
        runtime,
        "maze me",
        |c, t| c.get_maze_me(t),
    )?;
    let mut total = 0.0_f64;
    if let Some(active) = me.active_session.clone() {
        if active.status == "pending" || active.status.is_empty() {
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 发现迷宫{}残局（对局 {}），先继续走完。",
                runtime.email(),
                active.difficulty,
                active.session_id,
            ));
            let r = solve_and_walk(cancel_flag, state, runtime, active, interval)?;
            total += r;
        }
    }

    for difficulty in MAZE_DIFFICULTY_ORDER.iter() {
        ui::check_cancel(cancel_flag)?;
        let me = with_auth_retry_api_until_success(
            cancel_flag,
            state,
            runtime,
            "maze me",
            |c, t| c.get_maze_me(t),
        )?;
        let remaining = me.daily_plays_remaining.get(*difficulty).copied().unwrap_or(0);
        let reward = config.difficulties.get(*difficulty).map(|c| c.reward_amount).unwrap_or(0.0);
        if remaining <= 0 {
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 迷宫{}今天已经领取完了。",
                runtime.email(),
                difficulty,
            ));
            continue;
        }
        state.lock().unwrap().log.line_fmt(format_args!(
            "账号 {} 迷宫{}今天剩 {} 局（每局奖励 {}）。",
            runtime.email(),
            difficulty,
            remaining,
            format_amount(reward),
        ));
        for i in 0..remaining {
            ui::check_cancel(cancel_flag)?;
            let start = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "maze start",
                |c, t| c.start_maze(t, difficulty),
            )?;
            let r = solve_and_walk(cancel_flag, state, runtime, start.session, interval)?;
            total += r;
            state.lock().unwrap().log.line_fmt(format_args!(
                "账号 {} 迷宫{}第 {}/{} 局：奖励 {}",
                runtime.email(),
                difficulty,
                i + 1,
                remaining,
                format_amount(r),
            ));
        }
    }

    state.lock().unwrap().log.line_fmt(format_args!(
        "账号 {} 自动迷宫运行完成，本次累计奖励 {}。",
        runtime.email(),
        format_amount(total),
    ));
    Ok(total)
}

fn solve_and_walk(
    cancel_flag: &ui::CancelFlag,
    state: &Arc<Mutex<BatchState>>,
    runtime: &mut AccountRuntime,
    initial: MazeSession,
    interval: Duration,
) -> io::Result<f64> {
    let mut session = initial;
    let mut steps = 0;
    loop {
        if session.status == "won" || session.won {
            return Ok(session.reward_amount);
        }
        if matches!(session.status.as_str(), "failed" | "abandoned" | "lost") {
            return Ok(0.0);
        }
        let path = match crate::solver::maze::solve(&session) {
            Some(p) => p,
            None => {
                state.lock().unwrap().log.line_fmt(format_args!(
                    "账号 {} 迷宫求解失败：对局 {} 无解（理论上不应发生），放弃这局。",
                    runtime.email(),
                    session.session_id,
                ));
                let sid = session.session_id;
                let _ = with_auth_retry_api_until_success(
                    cancel_flag,
                    state,
                    runtime,
                    "maze abandon",
                    |c, t| c.abandon_simple(t, crate::api::MAZE_ABANDON_PATH, "/maze", sid),
                );
                return Ok(0.0);
            }
        };
        if path.is_empty() {
            return Ok(session.reward_amount);
        }
        let session_id = session.session_id;
        for d in path {
            ui::check_cancel(cancel_flag)?;
            ui::sleep_with_cancel(cancel_flag, interval)?;
            let resp = with_auth_retry_api_until_success(
                cancel_flag,
                state,
                runtime,
                "maze move",
                |c, t| c.move_maze(t, session_id, d.name()),
            )?;
            session = resp.session;
            steps += 1;
            if session.won || session.status == "won" || matches!(session.status.as_str(), "failed" | "abandoned" | "lost") {
                break;
            }
            if steps > 50_000 {
                return Ok(0.0);
            }
        }
    }
}
