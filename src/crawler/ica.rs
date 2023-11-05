use super::{parse_unit_price, CrawlResult, CrawlTask, StoreCrawler};
use anyhow::Result;
use async_trait::async_trait;
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct ICACrawler;

#[async_trait]
impl StoreCrawler for ICACrawler {
    async fn crawl_price(&self, task: CrawlTask) -> Result<CrawlResult> {
        let url = format!(
            "https://handlaprivatkund.ica.se/stores/{}/api/v4/products/bop?retailerProductId={}",
            task.butik_id.unwrap(),
            task.fetch_id
        );
        let response = reqwest::get(url).await?;
        let body: serde_json::Value = response.json().await?;
        let product = &body["entities"]["product"]
            .as_object()
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
            .1;
        let unit_price = product["price"]["unit"]["current"]["amount"]
            .as_str()
            .unwrap();
        let price = parse_unit_price(unit_price).unwrap();
        Ok(CrawlResult {
            id: task.id,
            price,
            member_price: None,
        })
    }

    fn crawl_delay() -> u64 {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_crawl_price() {
        let crawler = ICACrawler;
        let task = CrawlTask {
            id: 1,
            butik_name: "ICA Supermarket".to_string(),
            butik_id: Some("1004293".to_string()),
            fetch_id: "1203222".to_string(),
        };
        let result = crawler.crawl_price(task).await.unwrap();
        println!("{} kronor and {} ore", result.price.0, result.price.1);
        assert_eq!(result.id, 1);
        // should not panic
    }
}
