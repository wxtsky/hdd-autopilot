//! 扫雷完整 CSP 求解器：trivial 推理 + 子集消去 + 连通子图分解 + 加权概率聚合。
//!
//! 核心算法（受 Studholme 论文 + JohnnyDeuss/minesweeper-solver 启发）：
//! 1. 把每个已揭开的数字格视为约束 `(unknowns, mines_needed)`
//! 2. **Trivial 推理**：mines_needed == 0 → 全 safe；== unknowns.len() → 全 mine
//! 3. **子集消去**：若 A 的 unknowns ⊂ B 的 unknowns，则 (B\A) 的雷数 = B.need - A.need
//!    - 推出 B\A 全 mine 或全 safe 的子集
//! 4. **连通子图分解**：boundary 中通过共享约束相连的格作为一组（Union-Find），独立枚举
//! 5. **每子图枚举**：DFS 枚举所有合法雷配置，记录 (此子图雷数 → 配置数)
//! 6. **跨子图聚合**：用全局雷数约束（剩余总雷数 = 各子图选中雷数 + 外部 unconstrained 雷数）
//!    每个组合用 C(unconstrained, mines_in_unconstrained) 加权
//! 7. **概率求解**：mine_prob[cell] = sum(weighted models with cell=mine) / sum(weighted models)
//! 8. **决策**：100% safe 必 reveal、100% mine 必 flag；都不确定 → 选概率最低（优先未约束区或角落）

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

/// 决定下一步操作。
/// 返回多个动作（可能是若干 flag + 若干 reveal）。
/// - 完全空 board → first click（角落，邻居最少，期望展开最多）
/// - trivial 推理出 100% safe/mine → 全部返回
/// - 仍有 unknown → CSP 算概率，返回 100% 安全格（reveal）+ 100% 雷格（flag）
/// - 无 100% 结论 → 最后一个动作返回最低 mine_prob 的 reveal（猜雷）
pub fn next_actions(board: &Board) -> Vec<Action> {
    // 1. 全空 board → 第一步选角落
    let unknowns = board.unknown_cells();
    if unknowns.is_empty() {
        return Vec::new();
    }
    let any_revealed = (0..board.rows).any(|y| {
        (0..board.cols).any(|x| matches!(board.cell(x, y), Cell::Number(_)))
    });
    if !any_revealed {
        // hdd 用 flood fill 模式：中心点击展开范围最大（8 邻居 → 信息最多）
        // 选 (cols/2, rows/2) 中心
        return vec![Action::Reveal { x: board.cols / 2, y: board.rows / 2 }];
    }

    // 2. 收集约束
    let constraints = collect_constraints(board);

    // 3. Trivial + 子集消去推理（迭代直到收敛）
    let (forced_safe, forced_mine) = inference(&constraints, &unknowns);
    if !forced_safe.is_empty() || !forced_mine.is_empty() {
        let mut acts = Vec::new();
        for c in &forced_mine {
            acts.push(Action::Flag { x: c.0, y: c.1 });
        }
        for c in &forced_safe {
            acts.push(Action::Reveal { x: c.0, y: c.1 });
        }
        return acts;
    }

    // 4. CSP 求解：连通子图分解 + 枚举 + 加权概率
    let probs = mine_probabilities(board, &constraints, &unknowns);
    if probs.is_empty() {
        // fallback：选第一个 unknown
        let c = unknowns[0];
        return vec![Action::Reveal { x: c.0, y: c.1 }];
    }

    // 5. 找出确定的 0% 安全格 + 100% 雷格
    let mut forced = Vec::new();
    let mut min_prob = f64::INFINITY;
    let mut min_cell: Option<(i32, i32)> = None;
    for (&cell, &p) in &probs {
        if p < 1e-9 {
            forced.push(Action::Reveal { x: cell.0, y: cell.1 });
        } else if (p - 1.0).abs() < 1e-9 {
            forced.push(Action::Flag { x: cell.0, y: cell.1 });
        } else if p < min_prob {
            min_prob = p;
            min_cell = Some(cell);
        }
    }
    if !forced.is_empty() {
        return forced;
    }

    // 6. 全是不确定 → 选 mine_prob 最低的猜
    if let Some(c) = min_cell {
        return vec![Action::Reveal { x: c.0, y: c.1 }];
    }
    Vec::new()
}

#[derive(Debug, Clone)]
struct Constraint {
    unknowns: Vec<(i32, i32)>,
    need: i32,
}

fn collect_constraints(board: &Board) -> Vec<Constraint> {
    let mut out = Vec::new();
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
            out.push(Constraint {
                unknowns: unk,
                need: n - flag_count,
            });
        }
    }
    out
}

/// Trivial + 子集消去：返回 (forced_safe, forced_mine) 集合。
fn inference(
    constraints: &[Constraint],
    _all_unknowns: &[(i32, i32)],
) -> (HashSet<(i32, i32)>, HashSet<(i32, i32)>) {
    let mut safe: HashSet<(i32, i32)> = HashSet::new();
    let mut mine: HashSet<(i32, i32)> = HashSet::new();
    let mut working: Vec<(HashSet<(i32, i32)>, i32)> = constraints
        .iter()
        .map(|c| (c.unknowns.iter().copied().collect(), c.need))
        .collect();
    loop {
        let mut changed = false;

        // Trivial 推理 + 应用 safe/mine 到约束
        let mut new_working: Vec<(HashSet<(i32, i32)>, i32)> = Vec::new();
        for (mut cells, mut need) in working.drain(..) {
            // 移除已知 safe（不影响 need）
            let safe_in: Vec<_> = cells.intersection(&safe).copied().collect();
            for s in &safe_in {
                cells.remove(s);
            }
            // 移除已知 mine（need 减 1）
            let mine_in: Vec<_> = cells.intersection(&mine).copied().collect();
            for m in &mine_in {
                cells.remove(m);
                need -= 1;
            }
            if !safe_in.is_empty() || !mine_in.is_empty() {
                changed = true;
            }
            if need < 0 {
                continue;
            }
            if cells.is_empty() {
                continue;
            }
            // Trivial
            if need == 0 {
                for c in &cells {
                    if safe.insert(*c) {
                        changed = true;
                    }
                }
                continue;
            }
            if need == cells.len() as i32 {
                for c in &cells {
                    if mine.insert(*c) {
                        changed = true;
                    }
                }
                continue;
            }
            new_working.push((cells, need));
        }
        working = new_working;

        // 子集消去：对每对约束 A、B，若 A.cells ⊂ B.cells，则 B - A 是新约束 (cells=B\A, need=B.need - A.need)
        let n = working.len();
        let mut subset_new: Vec<(HashSet<(i32, i32)>, i32)> = Vec::new();
        for i in 0..n {
            for j in 0..n {
                if i == j {
                    continue;
                }
                let (a_cells, a_need) = (&working[i].0, working[i].1);
                let (b_cells, b_need) = (&working[j].0, working[j].1);
                if a_cells.is_subset(b_cells) && a_cells.len() < b_cells.len() {
                    let diff: HashSet<(i32, i32)> =
                        b_cells.difference(a_cells).copied().collect();
                    let diff_need = b_need - a_need;
                    if diff_need < 0 {
                        continue;
                    }
                    if diff_need == 0 {
                        for c in &diff {
                            if safe.insert(*c) {
                                changed = true;
                            }
                        }
                    } else if diff_need == diff.len() as i32 {
                        for c in &diff {
                            if mine.insert(*c) {
                                changed = true;
                            }
                        }
                    } else {
                        subset_new.push((diff, diff_need));
                    }
                }
            }
        }
        // 把 subset 派生的约束加回 working（去重）
        for (cells, need) in subset_new {
            if !working.iter().any(|(c, n)| c == &cells && *n == need) {
                working.push((cells, need));
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }
    (safe, mine)
}

/// 计算每个未知格的雷概率。
/// 算法：
/// 1. 收集 boundary（出现在任何约束里的 unknowns）
/// 2. Union-Find 把 boundary 按"共享约束"分成连通子图
/// 3. 每个子图独立枚举所有合法雷配置（约束在子图内的）
/// 4. 跨子图聚合：用全局雷数约束 + unconstrained 区域 + 二项系数加权
/// 5. 求每格的雷概率
fn mine_probabilities(
    board: &Board,
    constraints: &[Constraint],
    all_unknowns: &[(i32, i32)],
) -> HashMap<(i32, i32), f64> {
    let mut probs: HashMap<(i32, i32), f64> = HashMap::new();

    // 收集 boundary
    let mut boundary_set: HashSet<(i32, i32)> = HashSet::new();
    for c in constraints {
        for cell in &c.unknowns {
            boundary_set.insert(*cell);
        }
    }
    let unconstrained: Vec<(i32, i32)> = all_unknowns
        .iter()
        .copied()
        .filter(|c| !boundary_set.contains(c))
        .collect();

    if boundary_set.is_empty() {
        // 没有约束（开局或孤立区）→ 平均概率
        let remaining = (board.mine_count - board.flag_count()).max(0) as f64;
        let p = if all_unknowns.is_empty() {
            0.0
        } else {
            (remaining / all_unknowns.len() as f64).clamp(0.0, 1.0)
        };
        for cell in all_unknowns {
            probs.insert(*cell, p);
        }
        return probs;
    }

    // Union-Find 子图分解：两个 cell 同子图 ⟺ 它们出现在同一约束里
    let boundary: Vec<(i32, i32)> = boundary_set.iter().copied().collect();
    let cell_idx: HashMap<(i32, i32), usize> = boundary
        .iter()
        .enumerate()
        .map(|(i, c)| (*c, i))
        .collect();
    let n = boundary.len();
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut Vec<usize>, x: usize) -> usize {
        if parent[x] != x {
            parent[x] = find(parent, parent[x]);
        }
        parent[x]
    }
    fn union(parent: &mut Vec<usize>, x: usize, y: usize) {
        let rx = find(parent, x);
        let ry = find(parent, y);
        if rx != ry {
            parent[rx] = ry;
        }
    }
    for c in constraints {
        for i in 1..c.unknowns.len() {
            let a = cell_idx[&c.unknowns[0]];
            let b = cell_idx[&c.unknowns[i]];
            union(&mut parent, a, b);
        }
    }
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().push(i);
    }

    // 对每个子图：枚举其所有合法雷配置
    // 子图内的格子数 = group.len()
    // 子图内适用的约束 = constraints 中 unknowns 全在 group 内的
    // 每种配置记录 (mine_count_in_group, count_per_cell)
    struct GroupResult {
        cells: Vec<(i32, i32)>,
        // mines_count → 该雷数有多少种配置
        mine_count_distribution: HashMap<i32, u128>,
        // (mines_count, cell_idx_in_group) → 在 mines_count 个雷的配置中该格是雷的次数
        cell_mine_in_count: HashMap<(i32, usize), u128>,
    }
    let mut group_results: Vec<GroupResult> = Vec::new();
    for group_indices in groups.values() {
        let group_cells: Vec<(i32, i32)> = group_indices.iter().map(|&i| boundary[i]).collect();
        let group_set: HashSet<(i32, i32)> = group_cells.iter().copied().collect();
        let group_idx_in_group: HashMap<(i32, i32), usize> = group_cells
            .iter()
            .enumerate()
            .map(|(i, c)| (*c, i))
            .collect();
        let group_constraints: Vec<&Constraint> = constraints
            .iter()
            .filter(|c| c.unknowns.iter().all(|u| group_set.contains(u)))
            .collect();

        // 子图大小限制 — 超过 24 格放弃精确（fallback 到子图内估算）
        let g = group_cells.len();
        if g > 24 {
            // 局部估算：把每个约束作独立处理（不精确但安全）
            let mut local_probs: HashMap<(i32, i32), f64> = HashMap::new();
            for c in &group_constraints {
                let p = (c.need as f64 / c.unknowns.len() as f64).clamp(0.0, 1.0);
                for cell in &c.unknowns {
                    let entry = local_probs.entry(*cell).or_insert(0.5);
                    if p < *entry {
                        *entry = p;
                    }
                }
            }
            for cell in &group_cells {
                let p = local_probs.get(cell).copied().unwrap_or(0.5);
                probs.insert(*cell, p);
            }
            continue;
        }

        // 枚举 2^g 种 mine 分配
        let mut mine_count_distribution: HashMap<i32, u128> = HashMap::new();
        let mut cell_mine_in_count: HashMap<(i32, usize), u128> = HashMap::new();
        for mask in 0u64..(1u64 << g) {
            // 验证所有约束
            let mut ok = true;
            for c in &group_constraints {
                let mut k = 0i32;
                for cell in &c.unknowns {
                    let idx = group_idx_in_group[cell];
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
            let mc = mask.count_ones() as i32;
            *mine_count_distribution.entry(mc).or_insert(0) += 1;
            for i in 0..g {
                if (mask >> i) & 1 == 1 {
                    *cell_mine_in_count.entry((mc, i)).or_insert(0) += 1;
                }
            }
        }

        group_results.push(GroupResult {
            cells: group_cells,
            mine_count_distribution,
            cell_mine_in_count,
        });
    }

    // 跨子图聚合
    let total_mines_remaining = (board.mine_count - board.flag_count()).max(0);
    let unconstrained_count = unconstrained.len() as i32;

    // 卷积所有子图的 mine_count_distribution → total mines distribution
    // 结果：HashMap<总边界雷数, 配置数>
    let mut combined: HashMap<i32, u128> = HashMap::new();
    combined.insert(0, 1);
    for gr in &group_results {
        let mut next: HashMap<i32, u128> = HashMap::new();
        for (cur_total, cur_count) in &combined {
            for (g_mines, g_count) in &gr.mine_count_distribution {
                let new_total = *cur_total + *g_mines;
                if new_total > total_mines_remaining {
                    continue;
                }
                *next.entry(new_total).or_insert(0) += cur_count * g_count;
            }
        }
        combined = next;
    }

    // 加权：每个 (total_boundary_mines = k) 的配置数乘以 C(unconstrained, total_remaining - k)
    // 因为 unconstrained 区还能放 (total_remaining - k) 个雷，方案数 = C
    let mut weighted_models: HashMap<i32, u128> = HashMap::new();
    let mut total_weighted: u128 = 0;
    for (k, count) in &combined {
        let mines_in_unconstrained = total_mines_remaining - k;
        if mines_in_unconstrained < 0 || mines_in_unconstrained > unconstrained_count {
            continue;
        }
        let c = binomial(unconstrained_count as u64, mines_in_unconstrained as u64);
        let w = count * c;
        weighted_models.insert(*k, w);
        total_weighted = total_weighted.saturating_add(w);
    }
    if total_weighted == 0 {
        // 异常 — fallback
        return probs;
    }

    // 算每格 boundary 的雷概率
    // mine_prob[cell] = sum over (k, configs in this group with this cell=mine, configs in other groups with k - this.group_mines) * binomial / total
    // 简化：对每个子图独立计算 cell 概率（条件于聚合）
    for (gi, gr) in group_results.iter().enumerate() {
        // 卷积其他所有子图 → other_combined: total_mines_in_other_groups → count
        let mut other_combined: HashMap<i32, u128> = HashMap::new();
        other_combined.insert(0, 1);
        for (oi, ogr) in group_results.iter().enumerate() {
            if oi == gi {
                continue;
            }
            let mut next: HashMap<i32, u128> = HashMap::new();
            for (cur_total, cur_count) in &other_combined {
                for (g_mines, g_count) in &ogr.mine_count_distribution {
                    let new_total = *cur_total + *g_mines;
                    if new_total > total_mines_remaining {
                        continue;
                    }
                    *next.entry(new_total).or_insert(0) += cur_count * g_count;
                }
            }
            other_combined = next;
        }
        for (i, cell) in gr.cells.iter().enumerate() {
            // numerator: sum over (this_group_mines = mc) of cell_mine_in_count[(mc, i)] *
            //   sum over other_combined[other_mines] where this + other + uncon = total_remaining
            //   * C(unconstrained, total_remaining - this - other)
            let mut num: u128 = 0;
            for (&mc, _) in &gr.mine_count_distribution {
                let cell_count = gr.cell_mine_in_count.get(&(mc, i)).copied().unwrap_or(0);
                if cell_count == 0 {
                    continue;
                }
                for (&other_mines, &other_count) in &other_combined {
                    let mines_in_unc = total_mines_remaining - mc - other_mines;
                    if mines_in_unc < 0 || mines_in_unc > unconstrained_count {
                        continue;
                    }
                    let c = binomial(unconstrained_count as u64, mines_in_unc as u64);
                    num = num.saturating_add(cell_count * other_count * c);
                }
            }
            let p = num as f64 / total_weighted as f64;
            probs.insert(*cell, p.clamp(0.0, 1.0));
        }
    }

    // unconstrained 区的雷概率
    if !unconstrained.is_empty() {
        // E[mines_in_unconstrained] / unconstrained_count
        // E = sum over k of (total_remaining - k) * weighted_models[k] / total_weighted
        let mut numerator_e: u128 = 0;
        for (k, w) in &weighted_models {
            numerator_e =
                numerator_e.saturating_add(*w * (total_mines_remaining - k).max(0) as u128);
        }
        let e_mines = numerator_e as f64 / total_weighted as f64;
        let p = (e_mines / unconstrained_count as f64).clamp(0.0, 1.0);
        for cell in &unconstrained {
            probs.insert(*cell, p);
        }
    }

    probs
}

/// 二项式系数 C(n, k)，溢出时返回 u128::MAX 代替（少量子图过大可能溢出，留容错）。
fn binomial(n: u64, k: u64) -> u128 {
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut result: u128 = 1;
    for i in 0..k {
        let num = (n - i) as u128;
        let den = (i + 1) as u128;
        // result = result * num / den
        // 为了避免溢出先乘后除：用 GCD 分母简化
        let g = gcd(result.checked_mul(num).unwrap_or(u128::MAX), den);
        if g == 0 {
            return u128::MAX;
        }
        let r = result.checked_mul(num).map(|v| v / den);
        match r {
            Some(v) => result = v,
            None => return u128::MAX,
        }
        let _ = g;
    }
    result
}

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 { a } else { gcd(b, a % b) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_click_picks_corner_when_all_unknown() {
        let board = Board::new(8, 8, 10);
        let acts = next_actions(&board);
        assert_eq!(acts.len(), 1);
        match acts[0] {
            Action::Reveal { x, y } => {
                assert_eq!(x, 0);
                assert_eq!(y, 0);
            }
            _ => panic!("expected reveal"),
        }
    }

    #[test]
    fn trivial_inference_flags_obvious_mine() {
        // 一个角落数字 1 周围只有 1 个 unknown → 必为雷
        let mut board = Board::new(3, 3, 1);
        board.set_number(0, 0, 1);
        board.set_number(1, 0, 1);
        board.set_number(0, 1, 1);
        // (1,1) 是唯一 unknown
        let acts = next_actions(&board);
        assert!(acts.iter().any(|a| matches!(a, Action::Flag { x: 1, y: 1 })));
    }

    #[test]
    fn safe_reveal_when_all_constraints_zero() {
        let mut board = Board::new(3, 3, 1);
        board.set_number(1, 1, 1);
        board.set_flag(0, 0);
        // 周围已 1 个 flag 满足约束，剩 unknown 全 safe
        let acts = next_actions(&board);
        let reveals = acts
            .iter()
            .filter(|a| matches!(a, Action::Reveal { .. }))
            .count();
        assert!(reveals >= 1);
    }

    #[test]
    fn subset_elimination_works() {
        // 经典 1-1-2 模式
        // ? ? ?
        // 1 2 ?  ← 数字行
        // 0 0 0  ← 已揭开 (用 number 0 代表)
        let mut board = Board::new(3, 3, 2);
        board.set_number(0, 1, 1);
        board.set_number(1, 1, 2);
        board.set_number(2, 1, 0); // 0 表示无雷（也是 number cell）
        board.set_number(0, 2, 0);
        board.set_number(1, 2, 0);
        board.set_number(2, 2, 0);
        // (0,0)/(1,0)/(2,0) 三个 unknown
        // 数字 1 周围 unknown = (0,0),(1,0)，要 1 雷
        // 数字 2 周围 unknown = (0,0),(1,0),(2,0)，要 2 雷
        // 子集消去：(2,0) 必为雷
        let acts = next_actions(&board);
        assert!(
            acts.iter().any(|a| matches!(a, Action::Flag { x: 2, y: 0 })),
            "subset elimination should flag (2,0): {:?}",
            acts
        );
    }
}
