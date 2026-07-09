# 优化获取IP接口性能 Spec

## Why
`GET /cache/ip` 接口在高并发下响应极慢（可达数十秒），核心原因是每次请求都在热路径中执行 `check_ip`（3次 HTTP 代理验证调用，每次超时 8s），且使用 Redis `KEYS` 命令全量扫描。需要将 IP 验证逻辑从请求路径剥离，改为预验证 + 缓存直接返回。

## What Changes
- 移除 `get_ip` 热路径中的 `check_ip` 调用，直接返回已验证为存活的 IP
- 用 Redis `SCAN` 替代 `KEYS` 命令，避免阻塞 Redis
- 用 `SRANDMEMBER` 替代 `HGETALL` + 随机选择，O(1) 随机获取
- 将 `Arc<Mutex<ConnectionManager>>` 替换为 `fred` 连接池，消除锁竞争
- 修复 `while let Some` 无限循环 bug（无存活 IP 时死循环）
- 新增 `get_ip` 的快速路径：优先从 live IP 集合中随机返回

## Impact
- Affected specs: IP 缓存查询
- Affected code: `src/service/ip_cache.rs`, `src/main.rs`, `src/lib.rs`, `Cargo.toml`

## MODIFIED Requirements

### Requirement: 获取 IP 接口
系统 SHALL 提供高性能的 IP 获取接口，直接返回已验证为存活的代理 IP，不在请求路径中执行网络验证。

#### Scenario: 无筛选条件获取 IP
- **WHEN** 用户请求 `GET /cache/ip`
- **THEN** 系统从 Redis 的 live IP 集合中随机返回一个 IP，延迟 < 50ms

#### Scenario: 按协议类型筛选
- **WHEN** 用户请求 `GET /cache/ip?protocol_type=http`
- **THEN** 系统从对应协议的 live IP 集合中随机返回一个 IP

#### Scenario: 按协议类型和匿名级别筛选
- **WHEN** 用户请求 `GET /cache/ip?protocol_type=http&level=1`
- **THEN** 系统从对应协议和级别的 live IP 集合中随机返回一个 IP

#### Scenario: IP 池为空
- **WHEN** 系统无任何存活 IP
- **THEN** 返回错误码 404，消息 "ip pool is null"

### Requirement: IP 存活集合维护
系统 SHALL 在 `verify_task` 校验 IP 时同步维护一个按协议类型和级别分组的存活 IP 集合（使用 Redis Set），确保 `get_ip` 可快速随机获取。

#### Scenario: 校验通过
- **WHEN** `verify_task` 校验某个 IP 存活
- **THEN** 将该 IP 添加到对应 `ip_live::{protocol_type}::{level}` 的 Set 中

#### Scenario: 校验失败
- **WHEN** `verify_task` 校验某个 IP 不存活且未超过死亡阈值
- **THEN** 从对应 `ip_live::{protocol_type}::{level}` 的 Set 中移除该 IP

#### Scenario: 校验失败超过阈值
- **WHEN** `verify_task` 校验某个 IP 不存活且 `die_verify_count >= 10`
- **THEN** 从 Redis 中删除该 IP 的所有记录，同时从 live Set 中移除

### Requirement: 获取 IP 数量接口
系统 SHALL 提供 IP 数量的查询接口，直接返回 live Set 的大小，避免全量扫描。

#### Scenario: 查询 IP 总数
- **WHEN** 用户请求 `GET /cache/count`
- **THEN** 系统从 live Set 汇总计算总数，延迟 < 50ms