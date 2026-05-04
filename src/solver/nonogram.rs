//! Nonogram (Picross) 求解器：约束传播 + 回溯。
//!
//! 行/列 clue 给出该行/列的连续段长度。每行/列独立做约束推理：
//! 1. 枚举该行所有可能的填充组合（满足 clue）
//! 2. 与当前已知状态相容的组合保留
//! 3. 取所有保留组合的交集 → 哪些格必填、哪些必空
//! 反复迭代行和列直到没有新信息。如还有未确定格，选一个分叉点回溯。

use crate::model::NonogramSession;

const FILL: u8 = 1;
const CROSS: u8 = 2;
const UNKNOWN: u8 = 0;

/// 求解整个 nonogram。返回所有需要 fill 的 (r, c) 列表。
/// 失败返回 None。
pub fn solve(session: &NonogramSession) -> Option<Vec<(i32, i32)>> {
    let h = session.height as usize;
    let w = session.width as usize;
    if h == 0 || w == 0 || session.row_clues.len() != h || session.col_clues.len() != w {
        return None;
    }
    // 当前 board 状态
    let mut board: Vec<Vec<u8>> = (0..h)
        .map(|r| {
            (0..w)
                .map(|c| {
                    let v = session.cells.get(r).and_then(|row| row.get(c)).copied().unwrap_or(0);
                    match v {
                        1 => FILL,
                        2 => CROSS,
                        _ => UNKNOWN,
                    }
                })
                .collect()
        })
        .collect();
    let row_clues: Vec<Vec<i32>> = session.row_clues.clone();
    let col_clues: Vec<Vec<i32>> = session.col_clues.clone();

    if !solve_recursive(&mut board, &row_clues, &col_clues, h, w) {
        return None;
    }
    let mut clicks = Vec::new();
    for r in 0..h {
        for c in 0..w {
            if board[r][c] == FILL {
                let original = session
                    .cells
                    .get(r)
                    .and_then(|row| row.get(c))
                    .copied()
                    .unwrap_or(0);
                if original != 1 {
                    clicks.push((r as i32, c as i32));
                }
            }
        }
    }
    Some(clicks)
}

fn solve_recursive(
    board: &mut Vec<Vec<u8>>,
    row_clues: &[Vec<i32>],
    col_clues: &[Vec<i32>],
    h: usize,
    w: usize,
) -> bool {
    // 反复约束传播
    loop {
        let mut changed = false;
        // 行传播
        for r in 0..h {
            if !propagate_line(&mut board[r], &row_clues[r], &mut changed) {
                return false;
            }
        }
        // 列传播
        for c in 0..w {
            let mut col: Vec<u8> = (0..h).map(|r| board[r][c]).collect();
            if !propagate_line(&mut col, &col_clues[c], &mut changed) {
                return false;
            }
            for r in 0..h {
                board[r][c] = col[r];
            }
        }
        if !changed {
            break;
        }
    }
    // 找 unknown 分叉点
    let mut pick: Option<(usize, usize)> = None;
    for r in 0..h {
        for c in 0..w {
            if board[r][c] == UNKNOWN {
                pick = Some((r, c));
                break;
            }
        }
        if pick.is_some() {
            break;
        }
    }
    let Some((br, bc)) = pick else {
        // 全部确定，验证 clue 是否完全满足
        return verify(board, row_clues, col_clues, h, w);
    };
    for guess in [FILL, CROSS] {
        let mut copy = board.clone();
        copy[br][bc] = guess;
        if solve_recursive(&mut copy, row_clues, col_clues, h, w) {
            *board = copy;
            return true;
        }
    }
    false
}

/// 对单行做约束推理。给定当前 line（U/F/C 状态）和 clue。
/// 枚举所有满足 clue 且与 line 相容的填充组合，取交集回写 line。
/// 如果有新信息，set changed = true。如果一致组合数为 0，返回 false。
fn propagate_line(line: &mut Vec<u8>, clue: &[i32], changed: &mut bool) -> bool {
    let n = line.len();
    let total_filled: i32 = clue.iter().sum();
    let segments = clue.len() as i32;
    let min_len = total_filled + segments.saturating_sub(1);
    if min_len > n as i32 {
        return false;
    }
    // 收集所有可行填充：每个位置 0..n 的 u8
    let mut possible_fill = vec![0u32; n]; // 至少出现一次 FILL 的次数
    let mut possible_cross = vec![0u32; n];
    let mut count = 0u32;

    let mut current = vec![CROSS; n];
    let segs: Vec<usize> = clue.iter().map(|&x| x as usize).collect();
    enumerate(line, &segs, 0, 0, &mut current, &mut possible_fill, &mut possible_cross, &mut count);
    if count == 0 {
        return false;
    }
    // 如果某位置 fill 出现 count 次，则必定 FILL；如果 cross 出现 count 次，则必定 CROSS
    for i in 0..n {
        let must_fill = possible_fill[i] == count;
        let must_cross = possible_cross[i] == count;
        let new_state = if must_fill && !must_cross {
            FILL
        } else if must_cross && !must_fill {
            CROSS
        } else {
            UNKNOWN
        };
        if new_state != UNKNOWN && new_state != line[i] {
            if line[i] != UNKNOWN && line[i] != new_state {
                return false;
            }
            line[i] = new_state;
            *changed = true;
        }
    }
    true
}

/// 递归枚举一行的所有合法填充。
/// `seg_idx` 当前要放第几段，`pos` 从哪个位置开始尝试。
fn enumerate(
    line: &[u8],
    segs: &[usize],
    seg_idx: usize,
    pos: usize,
    current: &mut Vec<u8>,
    possible_fill: &mut [u32],
    possible_cross: &mut [u32],
    count: &mut u32,
) {
    let n = line.len();
    if seg_idx == segs.len() {
        // 把剩余位置全设 CROSS
        for i in pos..n {
            current[i] = CROSS;
        }
        // 检查与 line 兼容
        for i in 0..n {
            if line[i] != UNKNOWN && line[i] != current[i] {
                return;
            }
        }
        // 累计
        for i in 0..n {
            if current[i] == FILL {
                possible_fill[i] += 1;
            } else {
                possible_cross[i] += 1;
            }
        }
        *count += 1;
        return;
    }
    let seg_len = segs[seg_idx];
    let remaining_segs: usize = segs[seg_idx + 1..].iter().sum::<usize>() + (segs.len() - seg_idx - 1);
    let max_start = n.saturating_sub(seg_len + remaining_segs);
    for start in pos..=max_start {
        // 在 [pos..start] 放 CROSS
        for i in pos..start {
            current[i] = CROSS;
        }
        // 在 [start..start+seg_len] 放 FILL
        for i in start..start + seg_len {
            current[i] = FILL;
        }
        // 后接一个 CROSS（如果不是最后一段）
        let next_pos = if seg_idx + 1 < segs.len() {
            // 最后一格放 CROSS
            if start + seg_len < n {
                current[start + seg_len] = CROSS;
            }
            start + seg_len + 1
        } else {
            start + seg_len
        };
        // 提前剪枝：检查 [pos..next_pos] 范围与 line 兼容
        let upper = next_pos.min(n);
        let mut ok = true;
        for i in pos..upper {
            if line[i] != UNKNOWN && line[i] != current[i] {
                ok = false;
                break;
            }
        }
        if !ok {
            continue;
        }
        if next_pos <= n {
            enumerate(line, segs, seg_idx + 1, next_pos, current, possible_fill, possible_cross, count);
        }
    }
}

fn verify(
    board: &Vec<Vec<u8>>,
    row_clues: &[Vec<i32>],
    col_clues: &[Vec<i32>],
    h: usize,
    w: usize,
) -> bool {
    for r in 0..h {
        let line: Vec<u8> = board[r].clone();
        if !line_matches_clue(&line, &row_clues[r]) {
            return false;
        }
    }
    for c in 0..w {
        let line: Vec<u8> = (0..h).map(|r| board[r][c]).collect();
        if !line_matches_clue(&line, &col_clues[c]) {
            return false;
        }
    }
    true
}

fn line_matches_clue(line: &[u8], clue: &[i32]) -> bool {
    let mut segs = Vec::new();
    let mut cur = 0;
    for &cell in line {
        if cell == FILL {
            cur += 1;
        } else if cur > 0 {
            segs.push(cur);
            cur = 0;
        }
    }
    if cur > 0 {
        segs.push(cur);
    }
    segs.len() == clue.len() && segs.iter().zip(clue.iter()).all(|(a, b)| *a == *b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_simple_5x5() {
        // 简单例子：row_clues=[[5],[5],[5],[5],[5]], col_clues=[[5],[5],[5],[5],[5]] → 全 fill
        let session = NonogramSession {
            width: 5,
            height: 5,
            cells: vec![vec![0; 5]; 5],
            row_clues: vec![vec![5]; 5],
            col_clues: vec![vec![5]; 5],
            ..Default::default()
        };
        let clicks = solve(&session).expect("solvable");
        assert_eq!(clicks.len(), 25);
    }
}
