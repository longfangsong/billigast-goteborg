use super::{parse_unit_price, CrawlResult, CrawlTask, StoreCrawler};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::header::{HeaderMap, USER_AGENT};
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct WillysCrawler;

#[async_trait]
impl StoreCrawler for WillysCrawler {
    async fn crawl_price(&self, task: CrawlTask) -> Result<CrawlResult> {
        let url = format!("https://www.willys.se/axfood/rest/p/{}", task.fetch_id);
        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            "Mozilla/5.0 (X11; Linux x86_64; rv:10.0) Gecko/20100101 Firefox/10.0".parse()?,
        );
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
        let response = client.get(url).send().await?;
        let body: serde_json::Value = response.json().await?;
        let unit_price = body["comparePrice"].as_str().unwrap();
        let origin_price = parse_unit_price(unit_price).unwrap();
        let promotion_price = body["potentialPromotions"]
            .as_array()
            .and_then(|it| it.first())
            .and_then(|it| it.as_object())
            .and_then(|v| v["comparePrice"].as_str())
            .and_then(parse_unit_price);
        Ok(CrawlResult {
            id: task.id,
            price: origin_price,
            member_price: promotion_price,
        })
    }

    fn crawl_delay() -> u64 {
        10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_crawl_price() {
        let crawler = WillysCrawler;
        let task = CrawlTask {
            id: 1,
            butik_id: None,
            butik_name: "willys".to_string(),
            fetch_id: "101253220_KG".to_string(),
        };
        let result = crawler.crawl_price(task).await.unwrap();
        println!("{} kronor and {} ore", result.price.0, result.price.1);
        if let Some((k, o)) = result.member_price {
            println!("{} kronor and {} ore", k, o);
        }
        assert_eq!(result.id, 1);
        // should not panic
    }
}
