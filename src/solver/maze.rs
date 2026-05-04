//! 迷宫求解器：BFS 从起点 (0, 0) 走到 exit。
//!
//! `open_edges` 是无向边列表（两端都 [r, c]）。BFS 必胜（completeness 保证）。

use std::collections::{HashMap, HashSet, VecDeque};

use crate::model::MazeSession;

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn name(self) -> &'static str {
        match self {
            Direction::Up => "up",
            Direction::Down => "down",
            Direction::Left => "left",
            Direction::Right => "right",
        }
    }

    fn from_delta(dr: i32, dc: i32) -> Option<Self> {
        match (dr, dc) {
            (-1, 0) => Some(Direction::Up),
            (1, 0) => Some(Direction::Down),
            (0, -1) => Some(Direction::Left),
            (0, 1) => Some(Direction::Right),
            _ => None,
        }
    }
}

/// 用 BFS 从 player 当前位置走到 exit，返回方向序列。
pub fn solve(session: &MazeSession) -> Option<Vec<Direction>> {
    let start = (session.player[0], session.player[1]);
    let end = (session.exit[0], session.exit[1]);
    if start == end {
        return Some(Vec::new());
    }

    // 构造邻接表：cell -> set of neighbors
    let mut adj: HashMap<(i32, i32), Vec<(i32, i32)>> = HashMap::new();
    for edge in &session.open_edges {
        let a = (edge[0][0], edge[0][1]);
        let b = (edge[1][0], edge[1][1]);
        adj.entry(a).or_default().push(b);
        adj.entry(b).or_default().push(a);
    }

    let mut prev: HashMap<(i32, i32), (i32, i32)> = HashMap::new();
    let mut seen: HashSet<(i32, i32)> = HashSet::new();
    seen.insert(start);
    let mut queue: VecDeque<(i32, i32)> = VecDeque::new();
    queue.push_back(start);

    while let Some(cur) = queue.pop_front() {
        if cur == end {
            // 回溯路径，构造方向序列
            let mut path = Vec::new();
            let mut at = end;
            while at != start {
                let from = prev[&at];
                let dr = at.0 - from.0;
                let dc = at.1 - from.1;
                if let Some(d) = Direction::from_delta(dr, dc) {
                    path.push(d);
                } else {
                    return None;
                }
                at = from;
            }
            path.reverse();
            return Some(path);
        }
        if let Some(neighbors) = adj.get(&cur) {
            for &n in neighbors {
                if seen.insert(n) {
                    prev.insert(n, cur);
                    queue.push_back(n);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_trivial_two_cell_maze() {
        let session = MazeSession {
            session_id: 1,
            player: [0, 0],
            exit: [0, 1],
            open_edges: vec![[[0, 0], [0, 1]]],
            width: 2,
            height: 1,
            ..Default::default()
        };
        let path = solve(&session).expect("should solve");
        assert_eq!(path.len(), 1);
        assert_eq!(path[0].name(), "right");
    }

    #[test]
    fn returns_empty_when_already_at_exit() {
        let session = MazeSession {
            player: [3, 4],
            exit: [3, 4],
            ..Default::default()
        };
        assert!(solve(&session).unwrap().is_empty());
    }
}
