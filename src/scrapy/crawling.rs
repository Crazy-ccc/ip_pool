use crate::model::ip_detail::IpDetail;
use crate::scrapy::crawling_rule::CrawlingRule;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

fn extract_text(
    element: &scraper::ElementRef,
    rule: &str,
    replace_rules: &HashMap<String, String>,
) -> Option<String> {
    if rule.is_empty() {
        return Some(String::new());
    }
    let selector = Selector::parse(rule).ok()?;
    element
        .select(&selector)
        .next()
        .map(|el| el.text().collect::<Vec<_>>().concat().trim().to_string())
        .map(|mut str| {
            replace_rules
                .iter()
                .for_each(|(rule, replace)| str = str.replace(rule, replace));
            str
        })
}

fn normalize_protocol(text: &str) -> String {
    let lower = text.trim().to_lowercase();
    if lower == "http" || lower == "https" || lower == "socks4" || lower == "socks5" {
        return lower;
    }
    if lower.contains("socks5") || lower.contains("socks 5") {
        return "socks5".to_string();
    }
    if lower.contains("socks4") || lower.contains("socks 4") {
        return "socks4".to_string();
    }
    if lower.contains("https") {
        return "https".to_string();
    }
    "http".to_string()
}

fn normalize_level(text: &str) -> String {
    let cleaned = text.trim();
    match cleaned {
        "高匿" | "高匿名" | "高匿代理IP" => "1",
        "普匿" | "普通匿名" | "普通代理IP" => "2",
        "匿名" | "匿名代理" => "3",
        "透明" | "透明代理" | "无匿" | "透明代理IP" => "4",
        "未知" => "5",
        s if s.len() == 1 && s.starts_with(|c: char| c.is_ascii_digit()) => s,
        _ => "5",
    }
    .to_string()
}

pub async fn crawling(rule: &CrawlingRule) -> Vec<IpDetail> {
    let mut results = Vec::new();

    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return results,
    };

    for page in 1..=rule.max_page {
        let url = rule.url.replace("{page}", &page.to_string());

        let html = match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => match resp.text().await {
                Ok(t) => t,
                Err(_) => continue,
            },
            _ => continue,
        };

        let doc = Html::parse_document(&html);

        let table_selector = match Selector::parse(&rule.table_rule) {
            Ok(s) => s,
            Err(_) => continue,
        };

        for table in doc.select(&table_selector) {
            let ip = match extract_text(&table, &rule.ip_rule, &rule.replace_rules) {
                Some(ip) if !ip.is_empty() => ip,
                _ => continue,
            };

            results.push(IpDetail {
                ip,
                port: extract_text(&table, &rule.port_rule, &rule.replace_rules)
                    .unwrap_or_default(),
                protocol_type: extract_text(&table, &rule.protocol_type_rule, &rule.replace_rules)
                    .as_ref()
                    .map(|s| normalize_protocol(s))
                    .unwrap_or_default(),
                level: extract_text(&table, &rule.level_rule, &rule.replace_rules)
                    .as_ref()
                    .map(|s| normalize_level(s))
                    .unwrap_or_default(),
                region: extract_text(&table, &rule.region_rule, &HashMap::new())
                    .unwrap_or_default(),
                crawling_time: now_millis(),
                live_time: 0,
                is_live: true,
                verify_count: 0,
            });
        }
    }

    results
}

#[cfg(test)]
mod test {
    use super::crawling;
    use crate::scrapy::crawling_rule::CrawlingRule;

    #[actix_web::test]
    async fn crawling_test() {
        let json_str = include_str!("../../resource/crawling_rules.json");
        let rules: Vec<CrawlingRule> = serde_json::from_str(json_str).unwrap();

        for rule in &rules {
            let result = crawling(&rule).await;
            println!("{}", result.len());
            assert!(!result.is_empty(), "should crawl at least one ip for rule");
            for ip in &result {
                assert!(!ip.ip.is_empty(), "ip should not be empty");
                assert!(!ip.port.is_empty(), "port should not be empty");
            }
        }
    }
}
