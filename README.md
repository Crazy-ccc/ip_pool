# IP Pool

基于 Rust 的代理 IP 池系统，定时爬取免费代理网站，自动验证可用性并提供 HTTP API 获取代理。

## 功能

- **定时爬取**：从多个免费代理网站爬取代理 IP（谷德、66代理、快代理、齐云代理等）
- **自动验证**：定时验证代理可用性，失效代理自动剔除
- **HTTP API**：提供 RESTful API 获取可用代理，支持按协议类型和匿名度筛选
- **Docker 支持**：一键容器化部署

## 技术栈

| 组件 | 选型 |
|---|---|
| 语言 | Rust 2024 edition |
| Web 框架 | actix-web 4 |
| 数据存储 | Redis (Hash) |
| HTTP 客户端 | reqwest (支持 SOCKS) |
| HTML 解析 | scraper |

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

## API

服务启动在 `127.0.0.1:8080`，仅支持 GET 请求。

### 获取代理

```
GET /cache/ip
```

查询参数：

| 参数 | 类型 | 说明 | 默认值 |
|---|---|---|---|
| `protocol_type` | string | 代理协议：http / https / socks4 / socks5 | `http` |
| `level` | string | 匿名度：1(高匿) / 2(普匿) / 3(匿名) / 4(透明) / 5(未知) | `1` |

响应示例：

```json
{
  "code": 0,
  "msg": "success",
  "data": {
    "ip": "123.45.67.89",
    "port": "8080",
    "protocol_type": "http",
    "level": "1",
    "region": "中国 广东 深圳",
    "crawling_time": 1718000000000,
    "live_time": 600000,
    "is_live": true,
    "verify_count": 3
  }
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
  "msg": "success",
  "data": 128
}
```

## 配置

| 配置项 | 方式 | 默认值 |
|---|---|---|
| Redis 地址 | `REDIS_URL` 环境变量 | `redis://127.0.0.1:6379` |
| 日志级别 | `RUST_LOG` 环境变量 | `actix_web=info,ip_pool=info` |
| 爬取间隔 | 硬编码（`task.rs`） | 12 小时 |
| 验证间隔 | 硬编码（`task.rs`） | 10 分钟 |
| 并发上限 | 硬编码（`main.rs`） | 4 |
| 代理验证超时 | 硬编码（`ip_cache.rs`） | 5 秒 |
| 爬取超时 | 硬编码（`crawling.rs`） | 10 秒 |
| 爬取规则 | `resource/crawling_rules.json`（编译时嵌入） | 6 条规则 |

## 项目结构

```
├── Cargo.toml
├── Dockerfile
├── resource/
│   ├── crawling_rules.json   # 爬取规则（生效）
│   └── emply_rule.json       # 爬取规则（备用）
└── src/
    ├── main.rs               # 入口：启动 Redis 连接、后台任务、HTTP 服务
    ├── lib.rs                # 全局状态 AppState、通用响应 Resp
    ├── db/redis.rs           # Redis 连接管理
    ├── model/ip_detail.rs    # IpDetail 数据模型
    ├── scrapy/
    │   ├── crawling_rule.rs  # 爬取规则结构体
    │   └── crawling.rs       # 爬取引擎
    └── service/
        ├── pool.rs           # 信号量并发池
        ├── task.rs           # 后台爬取 + 验证任务
        └── ip_cache.rs       # API 路由 + Redis 缓存操作
```

## 数据模型

IpDetail 存储在 Redis Hash 中：

```
key:   ip_cache::{protocol_type}::{level}     （如 ip_cache::http::1）
field: {ip}:{port}                             （如 123.45.67.89:8080）
value: JSON 序列化的 IpDetail
```

```rust
pub struct IpDetail {
    pub ip: String,             // IP 地址
    pub port: String,           // 端口
    pub protocol_type: String,  // 协议：http / https / socks4 / socks5
    pub level: String,          // 匿名度：1~5
    pub region: String,         // 地区
    pub crawling_time: u64,     // 爬取时间戳（毫秒）
    pub live_time: u64,         // 有效时长（毫秒，0 表示不限制）
    pub is_live: bool,          // 是否存活
    pub verify_count: u32,      // 验证通过次数
}
```
