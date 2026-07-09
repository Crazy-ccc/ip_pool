# Tasks

- [x] Task 1: 替换 Redis 连接方式 — 将 `Arc<Mutex<ConnectionManager>>` 替换为 `fred` 连接池
  - [x] 在 `Cargo.toml` 中添加 `fred` 依赖，移除 `redis` 依赖
  - [x] 重写 `src/db/redis.rs`：使用 `fred` 创建连接池
  - [x] 修改 `src/main.rs`：初始化 `fred` 连接池，更新 `AppState`
  - [x] 修改 `src/lib.rs`：更新 `AppState` 类型
  - [x] 修改 `src/service/ip_cache.rs`：所有 Redis 操作改用 `fred` 异步接口
  - [x] 修改 `src/service/task.rs`：所有 Redis 操作改用 `fred` 异步接口

- [x] Task 2: 优化 `get_ip` 接口 — 移除热路径中的 `check_ip`，使用 live Set 快速返回
  - [x] 新增 `SRANDMEMBER` 从 live Set 随机获取 IP key，再 `HGET` 获取 IP 详情
  - [x] 无筛选条件时从 `ip_live::*` 的多个 Set 中随机选取
  - [x] 有 protocol_type 筛选时从 `ip_live::{protocol_type}::*` Set 中随机选取
  - [x] 有 protocol_type + level 筛选时从 `ip_live::{protocol_type}::{level}` Set 中直接获取
  - [x] 修复 `while let Some` 无限循环 bug — 改为 `if let Some`

- [x] Task 3: 优化 `get_count` 接口 — 使用 live Set 的 `SCARD` 替代全量扫描
  - [x] 汇总所有 `ip_live::*` 的 Set 大小作为总数

- [x] Task 4: 在 `verify_task` 中维护 live Set
  - [x] 校验通过时 `SADD` 到对应 live Set
  - [x] 校验失败但未超阈值时 `SREM` 从 live Set 移除
  - [x] 校验失败超过阈值时同时 `SREM` 和 `HDEL`
  - [x] 在 `crawl_task` 中爬取到新 IP 时也 `SADD` 到 live Set

- [x] Task 5: 用 `SCAN` 替代 `KEYS` 命令
  - [x] `get_all_ips` 中使用 `SCAN` 命令遍历所有 key
  - [x] `get_live_count` 改为使用 live Set 的 `SCARD`

# Task Dependencies
- Task 2 依赖 Task 1
- Task 3 依赖 Task 1
- Task 4 依赖 Task 1
- Task 5 依赖 Task 1