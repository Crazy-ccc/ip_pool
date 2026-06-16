use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct IpDetail {
    pub ip: String,
    pub port: String,
    // 代理协议，http，socks4，socks5，https
    pub protocol_type: String,
    // 1→高匿，2→普匿，3→匿名，4→透明，5→未知
    pub level: String,
    // 地区
    pub region: String,
    // 采集时间戳
    pub crawling_time: u64,
    // 存活时间（单位毫秒）
    pub live_time: u64,
    // 是否存活
    pub is_live: bool,
    // 校验次数
    pub verify_count: u32,
}

impl IpDetail {
    pub fn died(self: Self) -> Self {
        Self {
            is_live: false,
            verify_count: self.verify_count + 1,
            ..self
        }
    }
}
