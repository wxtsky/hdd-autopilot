//! 扫雷概率求解器：约束传播 + 局部子集枚举 + 全局概率推理。
//!
//! 棋盘状态以二维 grid 表示：
//! - Unknown：未揭开未标记
//! - Number(n)：已揭开，周围 n 个雷
//! - Flag：已标记为雷
//!
//! 求解策略：
//! 1. 先用 trivial 推理：某数字格周围 unknown 数 == 数字 - 已 flag 数 → 全是雷；
//!    某数字格周围已 flag 数 == 数字 → 剩余 unknown 全安全。
//! 2. 没有 trivial 结论时，对每个**约束子集**做枚举（局部）：找出所有可行雷分布，
//!    统计每格在多少种分布里是雷 → 概率。
//! 3. 全局未约束区域：用剩余雷数 / 未约束格数估计平均概率。
//! 4. 选概率最低的格 reveal（猜雷）。

use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Unknown,
    Number(u8),
    Flag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Reveal { x: i32, y: i32 },
    Flag { x: i32, y: i32 },
}

#[derive(Debug, Clone)]
pub struct Board {
    pub rows: i32,
    pub cols: i32,
    pub mine_count: i32,
    pub grid: Vec<Vec<Cell>>,
}

impl Board {
    pub fn new(rows: i32, cols: i32, mine_count: i32) -> Self {
        Self {
            rows,
            cols,
            mine_count,
            grid: vec![vec![Cell::Unknown; cols as usize]; rows as usize],
        }
    }

    /// 棋盘坐标系：服务端 click 用的是 (x, y)，但约束推理用 (row, col) 更直观。
    /// 这里我们规定：`grid[row][col]`，row=y，col=x（和服务端 reveal_cells 的 [x,y,n] 格式对应）。
    pub fn set_number(&mut self, x: i32, y: i32, n: u8) {
        if let Some(row) = self.grid.get_mut(y as usize) {
            if let Some(cell) = row.get_mut(x as usize) {
                *cell = Cell::Number(n);
            }
        }
    }

    pub fn set_flag(&mut self, x: i32, y: i32) {
        if let Some(row) = self.grid.get_mut(y as usize) {
            if let Some(cell) = row.get_mut(x as usize) {
                *cell = Cell::Flag;
            }
        }
    }

    pub fn set_unknown(&mut self, x: i32, y: i32) {
        if let Some(row) = self.grid.get_mut(y as usize) {
            if let Some(cell) = row.get_mut(x as usize) {
                if matches!(cell, Cell::Flag) {
                    *cell = Cell::Unknown;
                }
            }
        }
    }

    pub fn cell(&self, x: i32, y: i32) -> Cell {
        if x < 0 || y < 0 || x >= self.cols || y >= self.rows {
            return Cell::Unknown;
        }
        self.grid[y as usize][x as usize]
    }

    fn neighbors(&self, x: i32, y: i32) -> Vec<(i32, i32)> {
        let mut out = Vec::with_capacity(8);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && ny >= 0 && nx < self.cols && ny < self.rows {
                    out.push((nx, ny));
                }
            }
        }
        out
    }

    fn flag_count(&self) -> i32 {
        self.grid
            .iter()
            .flatten()
            .filter(|c| matches!(c, Cell::Flag))
            .count() as i32
    }

    fn unknown_cells(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for y in 0..self.rows {
            for x in 0..self.cols {
                if matches!(self.cell(x, y), Cell::Unknown) {
                    out.push((x, y));
                }
            }
        }
        out
    }
}

/// 决定下一步操作：返回多个动作（可能是若干 flag + 若干 reveal）。
/// 如果无 trivial 结论 → 算每格雷概率，选概率最低的格 reveal（最后一个动作）。
pub fn next_actions(board: &Board) -> Vec<Action> {
    let mut actions = trivial_actions(board);
    if !actions.is_empty() {
        return actions;
    }

    // 没确定结论 — 算概率猜雷
    let probs = mine_probabilities(board);
    if probs.is_empty() {
        // 全部 unknown — 第一次点击：选中心
        let cx = board.cols / 2;
        let cy = board.rows / 2;
        actions.push(Action::Reveal { x: cx, y: cy });
        return actions;
    }
    // 选概率最低的安全格
    if let Some((&(x, y), p)) = probs.iter().min_by(|a, b| {
        a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)
    }) {
        // 如果某个 cell p == 1.0 直接 flag
        if (*p - 1.0).abs() < 1e-9 {
            actions.push(Action::Flag { x, y });
            return actions;
        }
        actions.push(Action::Reveal { x, y });
    }
    actions
}

fn trivial_actions(board: &Board) -> Vec<Action> {
    let mut to_flag: HashSet<(i32, i32)> = HashSet::new();
    let mut to_reveal: HashSet<(i32, i32)> = HashSet::new();
    for y in 0..board.rows {
        for x in 0..board.cols {
            let n = match board.cell(x, y) {
                Cell::Number(n) => n as i32,
                _ => continue,
            };
            let neigh = board.neighbors(x, y);
            let unknown: Vec<(i32, i32)> = neigh
                .iter()
                .copied()
                .filter(|(nx, ny)| matches!(board.cell(*nx, *ny), Cell::Unknown))
                .collect();
            let flag_count = neigh
                .iter()
                .filter(|(nx, ny)| matches!(board.cell(*nx, *ny), Cell::Flag))
                .count() as i32;
            if unknown.is_empty() {
                continue;
            }
            // 周围还需 n - flag_count 个雷
            let need = n - flag_count;
            if need == unknown.len() as i32 {
                // 所有 unknown 都是雷
                for cell in &unknown {
                    to_flag.insert(*cell);
                }
            } else if need == 0 {
                // 周围 unknown 全部安全
                for cell in &unknown {
                    to_reveal.insert(*cell);
                }
            }
        }
    }
    let mut out: Vec<Action> = to_flag
        .into_iter()
        .map(|(x, y)| Action::Flag { x, y })
        .collect();
    out.extend(to_reveal.into_iter().map(|(x, y)| Action::Reveal { x, y }));
    out
}

/// 算每个未知格的雷概率（局部约束子集枚举 + 全局未约束估算）。
fn mine_probabilities(board: &Board) -> HashMap<(i32, i32), f64> {
    let unknowns = board.unknown_cells();
    if unknowns.is_empty() {
        return HashMap::new();
    }

    // 收集约束：每个数字格 → (相邻 unknown 集合, 还需雷数 = number - flag 数)
    struct Constraint {
        unknowns: Vec<(i32, i32)>,
        need: i32,
    }
    let mut constraints: Vec<Constraint> = Vec::new();
    for y in 0..board.rows {
        for x in 0..board.cols {
            let n = match board.cell(x, y) {
                Cell::Number(n) => n as i32,
                _ => continue,
            };
            let neigh = board.neighbors(x, y);
            let mut unk: Vec<(i32, i32)> = neigh
                .iter()
                .copied()
                .filter(|(nx, ny)| matches!(board.cell(*nx, *ny), Cell::Unknown))
                .collect();
            unk.sort();
            unk.dedup();
            if unk.is_empty() {
                continue;
            }
            let flag_count = neigh
                .iter()
                .filter(|(nx, ny)| matches!(board.cell(*nx, *ny), Cell::Flag))
                .count() as i32;
            constraints.push(Constraint {
                unknowns: unk,
                need: n - flag_count,
            });
        }
    }

    // 受约束格：所有出现在约束中的 unknown 格
    let mut constrained: Vec<(i32, i32)> = constraints
        .iter()
        .flat_map(|c| c.unknowns.iter().copied())
        .collect();
    constrained.sort();
    constrained.dedup();

    let mut probs: HashMap<(i32, i32), f64> = HashMap::new();
    for &cell in &unknowns {
        probs.insert(cell, 0.5); // 默认值
    }

    // 受约束格枚举（限制规模避免爆炸）
    let constrained_index: HashMap<(i32, i32), usize> = constrained
        .iter()
        .enumerate()
        .map(|(i, c)| (*c, i))
        .collect();
    let max_enum_bits = 22;
    if !constrained.is_empty() && constrained.len() <= max_enum_bits {
        let n = constrained.len();
        let mut count_mine = vec![0u64; n];
        let mut total_models = 0u64;
        let mut model_mine_count_sum: u64 = 0;
        for mask in 0u64..(1u64 << n) {
            // 验证所有约束
            let mut ok = true;
            for c in &constraints {
                let mut k = 0i32;
                for cell in &c.unknowns {
                    let idx = *constrained_index.get(cell).unwrap();
                    if (mask >> idx) & 1 == 1 {
                        k += 1;
                    }
                }
                if k != c.need {
                    ok = false;
                    break;
                }
            }
            if !ok {
                continue;
            }
            total_models += 1;
            let bits = mask.count_ones() as u64;
            model_mine_count_sum += bits;
            for i in 0..n {
                if (mask >> i) & 1 == 1 {
                    count_mine[i] += 1;
                }
            }
        }
        if total_models > 0 {
            for (i, cell) in constrained.iter().enumerate() {
                probs.insert(*cell, count_mine[i] as f64 / total_models as f64);
            }
            // 未约束区域：用剩余雷估计
            let remaining_mines =
                board.mine_count as f64 - board.flag_count() as f64
                    - (model_mine_count_sum as f64 / total_models as f64);
            let unconstrained: Vec<(i32, i32)> = unknowns
                .iter()
                .copied()
                .filter(|c| !constrained_index.contains_key(c))
                .collect();
            if !unconstrained.is_empty() {
                let p = (remaining_mines / unconstrained.len() as f64).clamp(0.0, 1.0);
                for cell in unconstrained {
                    probs.insert(cell, p);
                }
            }
        }
    } else if !constrained.is_empty() {
        // 受约束规模过大，对每个约束独立估算（粗略）
        for c in &constraints {
            let p = (c.need as f64 / c.unknowns.len() as f64).clamp(0.0, 1.0);
            for cell in &c.unknowns {
                let entry = probs.entry(*cell).or_insert(0.5);
                if p < *entry {
                    *entry = p;
                }
            }
        }
        // 未约束区域均匀估算
        let known_flagged = board.flag_count() as f64;
        let avg_constrained_mines: f64 = constraints
            .iter()
            .map(|c| c.need as f64 / c.unknowns.len() as f64 * c.unknowns.len() as f64)
            .sum::<f64>()
            / constraints.len().max(1) as f64;
        let remaining_mines =
            (board.mine_count as f64 - known_flagged - avg_constrained_mines).max(0.0);
        let unconstrained: Vec<(i32, i32)> = unknowns
            .iter()
            .copied()
            .filter(|c| !constrained_index.contains_key(c))
            .collect();
        if !unconstrained.is_empty() {
            let p = (remaining_mines / unconstrained.len() as f64).clamp(0.0, 1.0);
            for cell in unconstrained {
                probs.insert(cell, p);
            }
        }
    } else {
        // 完全没数字格 → 所有 unknown 概率相同
        let p = (board.mine_count as f64 - board.flag_count() as f64).max(0.0)
            / unknowns.len() as f64;
        for cell in &unknowns {
            probs.insert(*cell, p.clamp(0.0, 1.0));
        }
    }

    probs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_click_picks_center_when_all_unknown() {
        let board = Board::new(8, 8, 10);
        let acts = next_actions(&board);
        assert_eq!(acts.len(), 1);
        match acts[0] {
            Action::Reveal { x, y } => {
                assert_eq!(x, 4);
                assert_eq!(y, 4);
            }
            _ => panic!("expected reveal"),
        }
    }

    #[test]
    fn trivial_inference_flags_all_unknown_when_count_matches() {
        // 一个 1 数字格周围只有一个 unknown 格 → 必为雷
        let mut board = Board::new(3, 3, 1);
        board.set_number(0, 0, 1);
        // 周围 8 邻居：(1,0),(0,1),(1,1)，但 (0,0) 自己是数字
        // 让其他都是 number 0 除了 (1,1)
        board.set_number(1, 0, 1);
        board.set_number(0, 1, 1);
        // (1,1) 为 unknown — 推理：(0,0) 周围 unknown 仅 (1,1)，need=1 → flag
        let acts = next_actions(&board);
        assert!(acts.iter().any(|a| matches!(a, Action::Flag { x: 1, y: 1 })));
    }

    #[test]
    fn safe_reveal_when_count_zero_after_flags() {
        let mut board = Board::new(3, 3, 1);
        board.set_number(1, 1, 1);
        board.set_flag(0, 0);
        // 周围已经 1 个 flag，剩 7 个 unknown 全部安全
        let acts = next_actions(&board);
        let reveal_count = acts
            .iter()
            .filter(|a| matches!(a, Action::Reveal { .. }))
            .count();
        assert!(reveal_count >= 1);
    }
}
