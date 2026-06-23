# IP Pool

基于 Rust 的代理 IP 池系统，定时爬取免费代理网站，自动验证可用性并提供 HTTP API 获取代理。

## 功能

- **定时爬取**：从多个免费代理网站爬取代理 IP（谷德、66代理、快代理、齐云代理等）
- **自动验证**：定时验证代理可用性，失效代理自动剔除（连续 10 次检测失败后删除）
- **HTTP API**：提供 RESTful API 获取可用代理，支持按协议类型和匿名度筛选
- **信号量并发控制**：使用 `tokio::sync::Semaphore` 控制爬取与验证的并发数
- **Docker 支持**：一键容器化部署

## 技术栈

| 组件 | 选型 |
|---|---|
| 语言 | Rust 2024 edition |
| Web 框架 | actix-web 4 |
| 数据存储 | Redis (Hash) |
| HTTP 客户端 | reqwest（支持 SOCKS） |
| HTML 解析 | scraper |
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
docker build -t ip_pool .
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

查询参数：

| 参数 | 类型 | 说明 | 默认值 |
|---|---|---|---|
| `protocol_type` | string | 代理协议：http / https / socks4 / socks5 | 所有协议 |
| `level` | string | 匿名度：1(高匿) / 2(普匿) / 3(匿名) / 4(透明) / 5(未知) | 所有级别 |

> 仅当同时提供 `protocol_type` 和 `level` 时才会精确匹配到具体分组；只提供 `protocol_type` 时匹配该协议下的所有级别。

响应示例：

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

返回的代理会经过实时可用性验证，确保返回的 IP 真实可用。如无可用代理则返回：

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

响应示例：

```json
{
  "code": 0,
  "msg": "",
  "data": 128
}
```

返回所有协议和级别下的代理总数（包括已失效的）。

## 配置

| 配置项 | 方式 | 默认值 |
|---|---|---|
| Redis 地址 | `REDIS_URL` 环境变量 | `redis://127.0.0.1:6379` |
| 日志级别 | `RUST_LOG` 环境变量 | `actix_web=info,ip_pool=info` |
| 爬取间隔 | 硬编码（`task.rs`） | 12 小时 |
| 验证间隔 | 硬编码（`task.rs`） | 10 分钟 |
| 并发上限 | 硬编码（`main.rs`） | 4 |
| 代理验证超时 | 硬编码（`ip_cache.rs`） | 10 秒 |
| 爬取超时 | 硬编码（`crawling.rs`） | 10 秒 |
| 失败重启等待 | 硬编码（`task.rs`） | 60 秒 |
| 最大死亡验证次数 | 硬编码（`task.rs`） | 10 |
| 爬取规则 | `resource/crawling_rules.json` | 6 条规则 |

## 项目结构

```
├── Cargo.toml
├── Dockerfile
├── resource/
│   ├── crawling_rules.json   # 爬取规则（编译时嵌入）
│   └── emply_rule.json       # 爬取规则（备用，未使用）
└── src/
    ├── main.rs               # 入口：启动 Redis、后台任务、HTTP 服务
    ├── lib.rs                # AppState 全局状态、Resp 统一响应体
    ├── db/
    │   ├── mod.rs
    │   └── redis.rs          # Redis 连接管理（基于 ConnectionManager）
    ├── model/
    │   ├── mod.rs
    │   └── ip_detail.rs      # IpDetail 数据模型
    ├── scrapy/
    │   ├── mod.rs
    │   ├── crawling_rule.rs  # CrawlingRule 爬取规则结构体
    │   └── crawling.rs       # 爬取引擎：HTML 解析与字段提取
    └── service/
        ├── mod.rs
        ├── pool.rs           # Pool 信号量并发控制
        ├── ip_cache.rs       # API 路由 & Redis 缓存操作
        └── task.rs           # 后台爬取 + 验证定时任务
```

## 模块说明

### lib.rs — 全局状态 & 响应体

- `AppState`：全局共享状态，持有 `Arc<Mutex<ConnectionManager>>` 供各模块访问 Redis
- `Resp<T>`：统一 JSON 响应体，实现 `Responder` trait，自动序列化为 `{"code":0,"msg":"","data":...}`

### db/redis.rs — Redis 连接

从 `REDIS_URL` 环境变量读取连接地址，默认 `redis://127.0.0.1:6379`，返回 `redis::aio::ConnectionManager`。

### model/ip_detail.rs — 数据模型

```rust
pub struct IpDetail {
    pub ip: String,             // IP 地址
    pub port: String,           // 端口
    pub protocol_type: String,  // 协议：http / https / socks4 / socks5
    pub level: String,          // 匿名度：1=高匿 2=普匿 3=匿名 4=透明 5=未知
    pub region: String,         // 地区
    pub crawling_time: u64,     // 爬取时间戳（毫秒）
    pub live_time: u64,         // 有效时长（毫秒，0 表示不限制）
    pub is_live: bool,          // 是否存活
    pub verify_count: u32,      // 验证通过次数
    pub die_verify_count: u32,  // 连续死亡验证次数，超过 10 后删除
}
```

- `live()`：标记为存活，`verify_count +1`，`live_time + 10min`
- `died()`：标记为死亡，`die_verify_count +1`

### scrapy/crawling_rule.rs — 爬取规则

从 JSON 反序列化的爬取规则，包含站点名称、URL 模板（`{page}` 占位符）、最大页数、CSS 选择器规则和文本替换规则。

### scrapy/crawling.rs — 爬取引擎

- 使用 `reqwest` 请求目标页面，`scraper` 解析 HTML
- 按规则提取 IP、端口、协议类型、匿名度、地区
- 协议类型归一化：`http` / `https` / `socks4` / `socks5`
- 匿名度归一化：支持中文描述映射为 1~5 的数字
- 超时 10 秒

### service/pool.rs — 信号量并发池

基于 `tokio::sync::Semaphore` 实现，限制同时执行的任务数，防止对目标站点造成过大压力。

### service/ip_cache.rs — 缓存操作 & API

Redis Hash 存储结构：

```
key:   ip_cache::{protocol_type}::{level}     （如 ip_cache::http::1）
field: {ip}:{port}                             （如 123.45.67.89:8080）
value: JSON 序列化的 IpDetail
```

关键函数：

| 函数 | 可见性 | 说明 |
|---|---|---|
| `service()` | `pub` | 注册 `/cache/ip` 与 `/cache/count` 路由 |
| `check_ip()` | `pub` | 通过代理请求 `https://www.baidu.com` 验证可用性 |
| `ip_in_redis()` | `pub(crate)` | 将代理写入 Redis Hash |
| `remove_ip()` | `pub` | 从 Redis Hash 中删除代理 |
| `get_all_ips()` | `pub(crate)` | 获取所有缓存代理用于批量验证 |

### service/task.rs — 后台任务

`start()` 启动两个永久循环：

1. **爬取任务 (crawl_task)**：加载 `crawling_rules.json` 中的规则，逐站点爬取 → 验证 → 写入 Redis，完成后休眠 12 小时
2. **验证任务 (verify_task)**：每 10 分钟遍历 Redis 中所有代理，存活则更新 `live_time`，死亡计数递增，超过 10 次则删除
