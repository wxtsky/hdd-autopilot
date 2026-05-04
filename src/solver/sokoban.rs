use std::collections::{HashSet, VecDeque};

use crate::model::SokobanSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Pt {
    r: i32,
    c: i32,
}

impl Pt {
    fn shift(self, dr: i32, dc: i32) -> Self {
        Self {
            r: self.r + dr,
            c: self.c + dc,
        }
    }
}

const DIRS: &[(&str, i32, i32)] = &[
    ("up", -1, 0),
    ("down", 1, 0),
    ("left", 0, -1),
    ("right", 0, 1),
];

/// 用 BFS 求解推箱子。返回方向序列（"up"/"down"/"left"/"right"），无解返回 None。
pub fn solve(session: &SokobanSession) -> Option<Vec<&'static str>> {
    let walls: HashSet<Pt> = session
        .walls
        .iter()
        .map(|w| Pt { r: w[0], c: w[1] })
        .collect();
    let targets: HashSet<Pt> = session
        .targets
        .iter()
        .map(|t| Pt { r: t[0], c: t[1] })
        .collect();
    let start_player = Pt {
        r: session.player[0],
        c: session.player[1],
    };
    let mut start_boxes: Vec<Pt> = session
        .boxes
        .iter()
        .map(|b| Pt { r: b[0], c: b[1] })
        .collect();
    start_boxes.sort_by(|a, b| a.r.cmp(&b.r).then(a.c.cmp(&b.c)));

    if start_boxes.iter().all(|b| targets.contains(b)) {
        return Some(Vec::new());
    }

    let mut seen: HashSet<(Pt, Vec<Pt>)> = HashSet::new();
    seen.insert((start_player, start_boxes.clone()));
    let mut queue: VecDeque<(Pt, Vec<Pt>, Vec<&'static str>)> = VecDeque::new();
    queue.push_back((start_player, start_boxes, Vec::new()));

    let max_states = 200_000;
    while let Some((player, boxes, path)) = queue.pop_front() {
        if seen.len() > max_states {
            return None;
        }
        let box_set: HashSet<Pt> = boxes.iter().copied().collect();
        for (name, dr, dc) in DIRS {
            let np = player.shift(*dr, *dc);
            if walls.contains(&np) {
                continue;
            }
            let new_boxes = if box_set.contains(&np) {
                let bp = np.shift(*dr, *dc);
                if walls.contains(&bp) || box_set.contains(&bp) {
                    continue;
                }
                let mut nb: Vec<Pt> = boxes
                    .iter()
                    .map(|b| if *b == np { bp } else { *b })
                    .collect();
                nb.sort_by(|a, b| a.r.cmp(&b.r).then(a.c.cmp(&b.c)));
                if is_dead(&nb, &walls, &targets) {
                    continue;
                }
                nb
            } else {
                boxes.clone()
            };

            if !seen.insert((np, new_boxes.clone())) {
                continue;
            }
            let mut new_path = path.clone();
            new_path.push(*name);
            if new_boxes.iter().all(|b| targets.contains(b)) {
                return Some(new_path);
            }
            queue.push_back((np, new_boxes, new_path));
        }
    }
    None
}

/// 简单死锁检测：箱子被推到角落（两面是墙）且不在目标上。
fn is_dead(boxes: &[Pt], walls: &HashSet<Pt>, targets: &HashSet<Pt>) -> bool {
    for b in boxes {
        if targets.contains(b) {
            continue;
        }
        let up = walls.contains(&b.shift(-1, 0));
        let down = walls.contains(&b.shift(1, 0));
        let left = walls.contains(&b.shift(0, -1));
        let right = walls.contains(&b.shift(0, 1));
        if (up || down) && (left || right) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(walls: &[[i32; 2]], boxes: &[[i32; 2]], targets: &[[i32; 2]], player: [i32; 2]) -> SokobanSession {
        SokobanSession {
            session_id: 1,
            difficulty: "easy".into(),
            level_index: 0,
            status: "pending".into(),
            won: false,
            player,
            starting_player: player,
            boxes: boxes.to_vec(),
            starting_boxes: boxes.to_vec(),
            targets: targets.to_vec(),
            walls: walls.to_vec(),
            width: 9,
            height: 6,
            ..Default::default()
        }
    }

    #[test]
    fn solves_two_box_easy_level() {
        // 复刻实际 easy 第 2 关：player (3,4)；box (2,3),(2,4)；target (1,3),(1,4)
        let walls: Vec<[i32; 2]> = (0..9)
            .map(|c| [0, c])
            .chain((0..9).map(|c| [5, c]))
            .chain((0..6).map(|r| [r, 0]))
            .chain((0..6).map(|r| [r, 8]))
            .collect();
        let session = s(&walls, &[[2, 3], [2, 4]], &[[1, 3], [1, 4]], [3, 4]);
        let path = solve(&session).expect("solver finds a path");
        assert!(!path.is_empty());
        assert!(path.len() <= 10);
    }

    #[test]
    fn returns_empty_when_already_solved() {
        let session = s(&[], &[[1, 1]], &[[1, 1]], [2, 2]);
        let path = solve(&session).expect("already solved");
        assert!(path.is_empty());
    }
}
