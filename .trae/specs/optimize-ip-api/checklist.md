# Checklist

- [x] `get_ip` 接口不再调用 `check_ip`，直接返回 live Set 中的 IP
- [x] `get_ip` 使用 `SRANDMEMBER` 随机获取 IP，而非 `HGETALL` + 遍历
- [x] `get_ip` 支持 `protocol_type` 和 `level` 筛选参数
- [x] `get_count` 使用 live Set 的 `SCARD` 汇总，而非 `KEYS` + `HLEN`
- [x] `get_all_ips` 使用 `SCAN` 替代 `KEYS` 命令
- [x] `verify_task` 在校验后正确维护 live Set（SADD/SREM）
- [x] `crawl_task` 在爬取新 IP 后正确添加到 live Set
- [x] Redis 连接改为 `fred` 连接池，移除 `Arc<Mutex<ConnectionManager>>`
- [x] `while let Some` 无限循环 bug 已修复
- [x] 代码编译通过无错误