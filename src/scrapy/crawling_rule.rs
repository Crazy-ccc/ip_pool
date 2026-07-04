use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone)]
pub struct CrawlingRule {
    pub name: String,
    // 抓取地址
    pub url: String,
    // 最大抓取页数
    pub max_page: u8,
    // 最大抓取数量
    pub max_size: Option<usize>,
    // 数据源类型：空/"html" → HTML 爬取, "raw" → 原始文本下载
    #[serde(default)]
    pub source_type: String,
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
