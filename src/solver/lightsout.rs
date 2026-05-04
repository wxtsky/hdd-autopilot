//! Lights Out 求解器：GF(2) 高斯消元。
//!
//! N×N 棋盘共 N² 个格点，每格点击翻转自身 + 4 邻居。
//! 用矩阵 A (N²×N²) 表示影响：A[i][j] = 1 当点 j 影响格 i。
//! 已知初始状态 b（向量），求 x 使得 A·x = b (mod 2) — x 是哪些格需要点击。
//! 因为操作可交换且重复点 = 不点，x ∈ {0, 1}^{N²}。
//!
//! 实际上 A 是对称的（i 影响 j ⟺ j 影响 i），所以 A·x = b 等价于求 x。
//! 5×5 lightsout 矩阵秩 = 23（不满秩，有 2 维 kernel），所以解可能多个或零个。
//! 但 server 保证可解（scramble 自原始 0 状态），所以解一定存在。

use crate::model::LightsoutSession;

/// 求解 lightsout 棋盘，返回需要点击的 (r, c) 列表。
/// 如果服务端给定的 cells 状态可解，返回 Some(列表)；否则 None。
pub fn solve(session: &LightsoutSession) -> Option<Vec<(i32, i32)>> {
    let n = session.size as usize;
    if n == 0 || session.cells.len() != n {
        return None;
    }
    let nn = n * n;
    // b 向量：初始亮 = 1
    let mut b = vec![0u8; nn];
    for (r, row) in session.cells.iter().enumerate() {
        if row.len() != n {
            return None;
        }
        for (c, &v) in row.iter().enumerate() {
            b[r * n + c] = if v != 0 { 1 } else { 0 };
        }
    }
    // 全 0 已通关
    if b.iter().all(|x| *x == 0) {
        return Some(Vec::new());
    }
    // 构造矩阵 A (nn × nn)：A[i][j] = 1 当点 j 翻转格 i
    let mut a = vec![vec![0u8; nn]; nn];
    for r in 0..n {
        for c in 0..n {
            let idx = r * n + c;
            for &(dr, dc) in &[(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0 && nr < n as i32 && nc >= 0 && nc < n as i32 {
                    let neighbor_idx = (nr as usize) * n + (nc as usize);
                    // 点 idx 的格子，会翻转 neighbor_idx 这格
                    // 即 A[neighbor_idx][idx] = 1
                    a[neighbor_idx][idx] = 1;
                }
            }
        }
    }

    // 高斯消元 GF(2)：把 [A | b] 化简为行简化阶梯型
    // 增广矩阵：每行 nn+1 列（最后一列是 b）
    let mut aug = a;
    for i in 0..nn {
        aug[i].push(b[i]);
    }
    let cols = nn + 1;
    let mut row = 0;
    let mut pivot_col = vec![None; nn];
    for col in 0..nn {
        // 找该列在 row 之后第一个 1 行
        let mut piv = None;
        for r in row..nn {
            if aug[r][col] == 1 {
                piv = Some(r);
                break;
            }
        }
        let Some(piv) = piv else {
            continue;
        };
        aug.swap(row, piv);
        pivot_col[col] = Some(row);
        // 消去其他行该列
        for r in 0..nn {
            if r != row && aug[r][col] == 1 {
                for k in col..cols {
                    aug[r][k] ^= aug[row][k];
                }
            }
        }
        row += 1;
        if row == nn {
            break;
        }
    }
    // 检查是否一致：所有 0 = 非0 行 → 无解
    for r in row..nn {
        if aug[r][nn] != 0 {
            return None;
        }
    }
    // 构造 x：自由变量取 0
    let mut x = vec![0u8; nn];
    for col in 0..nn {
        if let Some(r) = pivot_col[col] {
            x[col] = aug[r][nn];
        }
    }
    let mut clicks = Vec::new();
    for i in 0..nn {
        if x[i] == 1 {
            let r = i / n;
            let c = i % n;
            clicks.push((r as i32, c as i32));
        }
    }
    Some(clicks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn already_solved_returns_empty() {
        let session = LightsoutSession {
            size: 3,
            cells: vec![vec![0; 3]; 3],
            ..Default::default()
        };
        assert!(solve(&session).unwrap().is_empty());
    }

    #[test]
    fn single_light_in_corner_solves() {
        // 5×5，只有 (0,0) 亮，按一次 (0,0) → 翻转 (0,0)/(0,1)/(1,0) → (0,0) 灭，(0,1)和(1,0) 亮 — 不行
        // 实际需要序列。让 solver 算出来：
        let mut cells = vec![vec![0; 5]; 5];
        cells[0][0] = 1;
        let session = LightsoutSession {
            size: 5,
            cells,
            ..Default::default()
        };
        let clicks = solve(&session).expect("should be solvable");
        // 只验证：模拟点击后全 0
        let mut sim = vec![vec![0i32; 5]; 5];
        sim[0][0] = 1;
        for (r, c) in clicks {
            for &(dr, dc) in &[(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                let nr = r + dr;
                let nc = c + dc;
                if nr >= 0 && nr < 5 && nc >= 0 && nc < 5 {
                    sim[nr as usize][nc as usize] ^= 1;
                }
            }
        }
        assert!(sim.iter().flatten().all(|x| *x == 0));
    }
}
