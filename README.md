# IP Pool

基于 Rust 的代理 IP 池系统，定时爬取免费代理网站，自动验证可用性并提供 HTTP API 获取可用代理。

## 功能

- **定时爬取**：从 18 个数据源（10 个 HTML 网站 + 8 个 GitHub 原始 TXT 文件）爬取代理 IP
- **自动验证**：每 10 分钟验证代理可用性，连续 10 次检测失败后自动删除
- **HTTP API**：提供 `GET /cache/ip` 和 `GET /cache/count` 接口，支持按协议和匿名度筛选
- **并发控制**：基于 `Semaphore` + `JoinSet` 的并发池，按完成顺序处理任务
- **实时校验**：每次返回代理前发起 3 路并发请求（百度 / httpbin / Google）验证可用性
- **Docker 支持**：多阶段构建，镜像仅 20MB+

## 技术栈

| 组件 | 选型 |
|---|---|
| 语言 | Rust 2024 edition |
| Web 框架 | actix-web 4 |
| 数据存储 | Redis (Hash) |
| HTTP 客户端 | reqwest（支持 SOCKS 代理） |
| HTML 解析 | scraper（CSS 选择器） |
| 运行时 | tokio |

## 快速开始

### 前置条件

- Rust 工具链（stable）
- Redis 服务（默认 `localhost:6379`）
- OpenSSL 开发库

### 构建与运行

```bash
# 构建
cargo build --release

# 配置 Redis 地址（可选，默认 redis://127.0.0.1:6379）
export REDIS_URL=redis://127.0.0.1:6379

# 配置日志级别（可选）
export RUST_LOG=actix_web=info,ip_pool=info

# 运行
cargo run --release
```

### Docker

```bash
# 构建镜像
docker build -t ip_pool .

# 运行（需确保 Redis 可访问）
docker run -e REDIS_URL=redis://host.docker.internal:6379 -p 8080:8080 ip_pool
```

### 测试

```bash
cargo test
```

> 注意：测试依赖外部代理网站可用性，需要网络连接。

## API

服务启动在 `127.0.0.1:8080`，仅支持 GET 请求，其余方法返回 `405 Method Not Allowed`。

### 获取代理

```
GET /cache/ip
```

**查询参数：**

| 参数 | 类型 | 说明 | 默认值 |
|---|---|---|---|
| `protocol_type` | string | 协议：`http` / `https` / `socks4` / `socks5` | 全部 |
| `level` | string | 匿名度：`1`(高匿) / `2`(普匿) / `3`(匿名) / `4`(透明) / `5`(未知) | 全部 |

参数组合行为：

| `protocol_type` | `level` | 匹配范围 |
|---|---|---|
| 无 | 无 | 所有协议 × 所有级别 |
| `http` | 无 | `ip_cache::http::*` 所有级别 |
| `http` | `1` | `ip_cache::http::1` 精确分组 |

**响应示例：**

```json
{
  "code": 0,
  "msg": "",
  "data": {
    "ip": "123.45.67.89",
    "port": "8080",
    "protocol_type": "http",
    "level": "1",
    "region": "中国 广东 深圳",
    "crawling_time": 1718000000000,
    "live_time": 600000,
    "is_live": true,
    "verify_count": 3,
    "die_verify_count": 0
  }
}
```

返回的代理经过实时可用性验证，确保真实可用。如无可用代理则返回：

```json
{
  "code": 404,
  "msg": "ip pool is null",
  "data": null
}
```

### 获取数量

```
GET /cache/count
```

返回 Redis 中所有缓存代理的总数（含失效）：

```json
{
  "code": 0,
  "msg": "",
  "data": 128
}
```

## Redis 数据模型

```
Key:     ip_cache::{protocol_type}::{level}
         例如: ip_cache::http::1
Field:   {ip}:{port}
         例如: 123.45.67.89:8080
Value:   JSON 序列化的 IpDetail
```

## 数据源

### HTML 源（10 个）

| 站点（名称） | URL | 页数 |
|---|---|---|
| 谷德 | goodips.com | 2 |
| 66 代理 | 66daili.com | 2 |
| 快代理-国内1 | kuaidaili.com/dps | 1 |
| 快代理-国内2 | kuaidaili.com/inha | 1 |
| 站大爷-国外 | zdaye.com/free_haiwai | 2 |
| 快代理-国外 | kuaidaili.com/fps | 1 |
| 齐云代理 | qiyunip.com | 1 |
| 站大爷-国内 | zdaye.com/free | 1 |

### Raw TXT 源（8 个）

| 名称 | 协议 |
|---|---|
| SyscallH00k HTTP / HTTPS / SOCKS4 / SOCKS5 | 4 |
| Thordata HTTP / HTTPS / SOCKS4 / SOCKS5 | 4 |

## 配置

| 配置项 | 方式 | 默认值 |
|---|---|---|
| Redis 地址 | 环境变量 `REDIS_URL` | `redis://127.0.0.1:6379` |
| 日志级别 | 环境变量 `RUST_LOG` | `actix_web=info,ip_pool=info` |
| 并发上限 | 硬编码（`main.rs`） | 4 |
| 爬取间隔 | 硬编码（`task.rs`） | 每 72 次验证循环（约 12h） |
| 验证间隔 | 硬编码（`task.rs`） | 10 分钟 |
| 代理验证超时 | 硬编码（`ip_cache.rs`） | 8 秒 |
| 连接超时 | 硬编码（`ip_cache.rs`） | 3 秒 |
| 爬取超时（HTML） | 硬编码（`crawling.rs`） | 10 秒 |
| 爬取超时（Raw） | 硬编码（`crawling.rs`） | 15 秒 |
| 最大死亡验证次数 | 硬编码（`task.rs`） | 10 |
| 爬取规则 | `resource/crawling_rules.json`（编译时嵌入） | 18 条 |

## 项目结构

```
├── Cargo.toml
├── Dockerfile                          # 多阶段构建
├── resource/
│   ├── crawling_rules.json             # 爬取规则（编译时嵌入）
│   └── emply_rule.json                 # 备用模板
└── src/
    ├── main.rs                         # 入口：Redis 连接、启动后台任务、HTTP 服务
    ├── lib.rs                          # AppState、Resp 统一响应体
    ├── db/
    │   └── redis.rs                    # Redis 连接管理（ConnectionManager）
    ├── model/
    │   └── ip_detail.rs                # IpDetail 数据模型（含 live/died 方法）
    ├── scrapy/
    │   ├── crawling_rule.rs            # CrawlingRule 爬取规则结构体
    │   └── crawling.rs                 # 爬取引擎：HTML + Raw TXT 解析
    └── service/
        ├── pool.rs                     # Semaphore + JoinSet 并发池
        ├── ip_cache.rs                 # Redis 缓存操作 + HTTP 路由处理
        └── task.rs                     # 后台爬取 + 验证定时任务
```

## 模块说明

### `lib.rs` — 全局状态 & 响应体

- `AppState`：全局共享状态，持有 `Arc<Mutex<ConnectionManager>>` 供各模块访问 Redis
- `Resp<T>`：统一 JSON 响应体，实现 `Responder` trait，自动序列化为 `{"code":0,"msg":"","data":...}`

### `db/redis.rs` — Redis 连接

从 `REDIS_URL` 环境变量读取连接地址，默认 `redis://127.0.0.1:6379`。

### `model/ip_detail.rs` — 数据模型

```rust
pub struct IpDetail {
    pub ip: String,
    pub port: String,
    pub protocol_type: String,  // http / https / socks4 / socks5
    pub level: String,          // 1=高匿 2=普匿 3=匿名 4=透明 5=未知
    pub region: String,
    pub crawling_time: u64,     // 爬取时间戳（毫秒）
    pub live_time: u64,         // 有效时长（毫秒，0=不限）
    pub is_live: bool,
    pub verify_count: u32,
    pub die_verify_count: u32,  // >10 时删除
}
```

- `live()` — 标记为存活，`verify_count +1`，`live_time + 10min`，`die_verify_count` 归零
- `died()` — 标记为死亡，`die_verify_count +1`，`live_time` 归零

### `scrapy/crawling.rs` — 爬取引擎

- 根据 `source_type` 分发到 `crawling_html()`（CSS 选择器解析）或 `crawling_raw()`（按行解析 TXT）
- 支持 `{page}` URL 模板占位符，自动遍历多页
- 协议归一化：`http` / `https` / `socks4` / `socks5`
- 匿名度归一化：中文描述 → 数字 1~5
- 文本替换规则：通过 `replace_rules` 清洗爬取内容

### `service/pool.rs` — 信号量并发池

组合 `Semaphore`（容量控制）与 `JoinSet`（完成顺序管理）。

- `spawn()` — 获取信号量许可后提交任务
- `join()` — 通过 `join_next()` 按完成顺序轮询，不会阻塞于前序未完成任务

### `service/ip_cache.rs` — 缓存操作 & API

| 函数 | 可见性 | 说明 |
|---|---|---|
| `service()` | `pub` | 注册 `/cache/ip` 与 `/cache/count` 路由 |
| `check_ip()` | `pub` | 格式预检 + 并发请求 3 个目标验证代理 |
| `ip_in_redis()` | `pub(crate)` | 写入 Redis Hash |
| `remove_ip()` | `pub` | 从 Redis Hash 删除 |
| `get_all_ips()` | `pub(crate)` | 获取所有缓存代理 |

### `service/task.rs` — 后台任务

`start()` 启动两个永久循环：

1. **爬取**：加载 `crawling_rules.json` 中 18 个规则，逐源爬取 → 验证 → 写入 Redis。完成后休眠约 12 小时；若可用 IP 为 0 则立即触发。
2. **验证**：每 10 分钟遍历 Redis 中所有代理，存活则更新 `live_time`，死亡计数递增，超过 10 次则删除。

## 设计要点

- **资源共享**：`Arc<Mutex<ConnectionManager>>` 让所有异步任务安全共享同一个 Redis 连接
- **规则编译嵌入**：`include_bytes!` 将爬取规则编译进二进制，运行时无需外部配置文件
- **多阶段构建**：`rust:alpine` 编译 → `alpine:3.21` 运行，`nobody` 用户运行，安全轻量
