# hdd-autopilot 快速上手

号多多公益站（sub.hdd.sb）白嫖游戏自动化 CLI（Rust）。每日跑一次自动玩完所有白嫖游戏，每号约 +150 余额/天。

## 1. 装 Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustc --version  # 应输出版本号
```

macOS 还要装 Xcode 命令行工具：
```bash
xcode-select --install
```

## 2. 编译

```bash
cd hdd-autopilot
cargo build --release --bin hdd-autopilot
```

首次编译约 1-3 分钟。失败一般是缺 C 编译器或 cmake，按错误提示装即可（可选 GPU 后端缺失会自动降级，不影响白嫖）。

编译产物：`target/release/hdd-autopilot`

## 3. 配置账号

第一次跑会让你手动输入邮箱密码，但**推荐预填 `var/data/auth.json`** 跳过交互：

```bash
mkdir -p var/data
chmod 700 var var/data
cat > var/data/auth.json <<'EOF'
{
  "base_url": "https://sub.hdd.sb",
  "accounts": [
    {
      "email": "你的邮箱@example.com",
      "password": "你的密码"
    }
  ]
}
EOF
chmod 600 var/data/auth.json
```

支持多账号：accounts 数组里加多个对象，会**并发**跑。

## 4. 跑

**一条命令跑全自动白嫖**（推荐每日复用）：

```bash
printf '2\n2\n1\n1\n14\n' | ./target/release/hdd-autopilot
```

菜单序列说明：
- `2` 主菜单 → 「需要登录的多账号批量操作功能」
- `2` 批量菜单 → 「账号添加完成，选择脚本功能」
- `1` 功能菜单 → **「白嫖玩法」**（**注意：2 是赌狗玩法，不要选**）
- `1` 白嫖菜单 → 「全自动运行所有白嫖玩法」
- `14` → 退出脚本

## 5. 包含的白嫖游戏（11 个）

每个都是 server-authoritative，每次胜利都给奖励：

| 游戏 | 求解器 | 胜率上限 | 单日满额 |
|------|--------|---------|--------|
| 签到 | - | 100% | 10-30 |
| 推箱子 sokoban | BFS | 100% | 14.5 |
| 迷宫 maze | BFS | 100% | 14.5 |
| 点灯 lightsout | GF(2) 高斯消元 | 100% | 14.5 |
| 数织 nonogram | 行列约束 + 回溯 | ~90% | ~13 |
| 数独 sudoku | 标准 solver | 100% | 15.2 |
| 华容道 puzzle15 | 滑块搜索 | 100% | 14 |
| 记忆翻牌 memory | 记忆 | 100% | 15 |
| 羊了个羊 tile | 队列搜索 | 100% | ~50 |
| 谜题 2048 | expectimax | ~95% | 15-30 |
| 扫雷 minesweeper | CSP 概率推理 | 53-90%（数学限制） | 7.5（30 局/天 配额）|

**单号每日合计 ~150 余额**。多号并发可线性叠加。

## 6. 单独玩某个游戏

不想跑全自动，可走单独菜单：

```
2. 自动签到
3. 自动羊了个羊
4. 自动谜题2048
5. 自动记忆翻牌
6. 自动华容道
7. 自动数独
8. 自动推箱子
9. 自动扫雷
10. 自动迷宫
11. 自动点灯
12. 自动数织
```

例如单跑数独：`printf '2\n2\n1\n7\n14\n' | ./target/release/hdd-autopilot`

## 7. 注意事项

- 🔴 **不要选「赌狗玩法」**（菜单 `2`）— 会消耗余额（刮刮乐等）
- 🔴 **不要跑挖矿**（主菜单 `1`）— 跑得慢且不稳定
- ⚠️ `var/data/auth.json` **明文存账号密码**，权限 600，不要 commit / 不要在公共机器跑
- ⚠️ 服务端有反作弊：跑太多可能被限 max_plays_per_day（扫雷已从 100 降到 30/天/号）
- ⚠️ 全自动一次约 25-30 分钟（2048 慢），可以挂着干别的

## 8. 日志

- 各游戏日志：`var/log/<game>/<email>.log`
- 余额变化：脚本输出实时打印
- 历史成绩：`https://sub.hdd.sb/<game>?ui_mode=embedded` 网页可看

## 9. 上游

代码 fork 自 [zyxtoworld/hdd-autopilot](https://github.com/zyxtoworld/hdd-autopilot)（Rust workspace + GPU 挖矿）。本 fork 加了 5 个游戏的 workflow（sokoban / minesweeper / maze / lightsout / nonogram）。

GitHub: https://github.com/wxtsky/hdd-autopilot
