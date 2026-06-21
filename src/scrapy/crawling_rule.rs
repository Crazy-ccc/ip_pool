use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct CrawlingRule {
    pub name: String,
    // 抓取地址
    pub url: String,
    // 最大抓取页数
    pub max_page: u8,
    // table
    pub table_rule: String,
    // ip
    pub ip_rule: String,
    // port
    pub port_rule: String,
    // protocol_type
    pub protocol_type_rule: String,
    // level
    pub level_rule: String,
    // region
    pub region_rule: String,
    // 替换规则
    pub replace_rules: HashMap<String, String>,
}
