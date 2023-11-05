use std::mem;

use super::{parse_unit_price, CrawlResult, CrawlTask, StoreCrawler};
use anyhow::Result;
use async_trait::async_trait;
use reqwest::{header::USER_AGENT, Client};
use serde_json::json;
use tokio::sync::mpsc;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct CoopCrawler;

#[async_trait]
impl StoreCrawler for CoopCrawler {
    async fn crawl_price(&self, task: CrawlTask) -> Result<CrawlResult> {
        let client = Client::new();
        let url = format!("https://external.api.coop.se/personalization/search/entities/by-id?api-version=v1&store={}&groups=CUSTOMER_PRIVATE&direct=false", task.butik_id.unwrap());
        let response = client
            .post(url)
            .header(
                "Ocp-Apim-Subscription-Key",
                "3becf0ce306f41a1ae94077c16798187",
            )
            .header("Content-Type", "application/json")
            .header(
                USER_AGENT,
                "Mozilla/5.0 (X11; Linux x86_64; rv:10.0) Gecko/20100101 Firefox/10.0",
            )
            .body(serde_json::to_string(&json!([task.fetch_id]))?)
            .send()
            .await?;
        let body: serde_json::Value = response.json().await?;
        let product = &body["results"]["items"][0].as_object().unwrap();
        let normal_price = product["piecePrice"].as_f64().unwrap();
        let member_price = product["promotionPrice"].as_f64();
        let current_comparative_price = product["comparativePrice"].as_f64().unwrap();
        let (normal_unit_price, member_unit_price) = if let Some(member_price) = member_price {
            let amount = member_price / current_comparative_price;
            (normal_price / amount, Some(current_comparative_price))
        } else {
            (current_comparative_price, None)
        };

        let unit_price = format!("{:.2}", normal_unit_price);
        let price = parse_unit_price(&unit_price).unwrap();
        let member_unit_price = member_unit_price.map(|it| format!("{:.2}", it));
        let member_price = member_unit_price.map(|it| parse_unit_price(&it).unwrap());
        Ok(CrawlResult {
            id: task.id,
            price,
            member_price,
        })
    }

    async fn start_crawler(
        &self,
        mut input_channel: mpsc::Receiver<CrawlTask>,
    ) -> mpsc::Receiver<Result<CrawlResult>> {
        let (tx, rx) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut tasks = Vec::new();
            let mut fetch_ids = Vec::new();
            while let Some(mut task) = input_channel.recv().await {
                fetch_ids.push(mem::take(&mut task.fetch_id));
                tasks.push(task);
            }
            let payloads_str = serde_json::to_string(&fetch_ids).unwrap();
            drop(fetch_ids);
            if tasks.len() == 0 {
                return;
            }
            let client = Client::new();
            let url = format!("https://external.api.coop.se/personalization/search/entities/by-id?api-version=v1&store={}&groups=CUSTOMER_PRIVATE&direct=false", tasks[0].butik_id.clone().unwrap());
            let response = client
                .post(url)
                .header(
                    "Ocp-Apim-Subscription-Key",
                    "3becf0ce306f41a1ae94077c16798187",
                )
                .header("Content-Type", "application/json")
                .header(
                    USER_AGENT,
                    "Mozilla/5.0 (X11; Linux x86_64; rv:10.0) Gecko/20100101 Firefox/10.0",
                )
                .body(payloads_str)
                .send()
                .await
                .unwrap();
            let body: serde_json::Value = response.json().await.unwrap();
            let items = body["results"]["items"]
                .as_array()
                .unwrap()
                .into_iter()
                .zip(tasks.into_iter())
                .map(|(product, task)| {
                    let normal_price = product["piecePrice"].as_f64().unwrap();
                    let member_price = product["promotionPrice"].as_f64();
                    let current_comparative_price = product["comparativePrice"].as_f64().unwrap();
                    let (normal_unit_price, member_unit_price) =
                        if let Some(member_price) = member_price {
                            let amount = member_price / current_comparative_price;
                            (normal_price / amount, Some(current_comparative_price))
                        } else {
                            (current_comparative_price, None)
                        };

                    let unit_price = format!("{:.2}", normal_unit_price);
                    let price = parse_unit_price(&unit_price).unwrap();
                    let member_unit_price = member_unit_price.map(|it| format!("{:.2}", it));
                    let member_price = member_unit_price.map(|it| parse_unit_price(&it).unwrap());
                    Ok(CrawlResult {
                        id: task.id,
                        price,
                        member_price,
                    })
                });
            for item in items {
                tx.send(item).await.unwrap();
            }
        });
        rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_crawl_price() {
        let payload = "8711327343573";
        let crawler = super::CoopCrawler;
        let task = CrawlTask {
            id: 1,
            butik_name: "coop".to_string(),
            butik_id: Some("252600".to_string()),
            fetch_id: payload.to_string(),
        };
        let result = crawler.crawl_price(task).await.unwrap();
        println!("{} kronor and {} ore", result.price.0, result.price.1);
        if let Some((kr, ore)) = result.member_price {
            println!("{} kronor and {} ore", kr, ore);
        }
        assert_eq!(result.id, 1);
        // should not panic
    }

    #[tokio::test]
    async fn test_start_crawler() {
        let crawler = super::CoopCrawler;
        let (tx, rx) = mpsc::channel(32);
        let task = CrawlTask {
            id: 1,
            butik_id: Some("252600".to_string()),
            butik_name: "coop".to_string(),
            fetch_id: "8711327343573".to_string(),
        };
        tx.send(task).await.unwrap();
        let task = CrawlTask {
            id: 2,
            butik_id: Some("252600".to_string()),
            butik_name: "coop".to_string(),
            fetch_id: "7312331808211".to_string(),
        };
        tx.send(task).await.unwrap();
        drop(tx);
        let mut receiver = crawler.start_crawler(rx).await;
        let result = receiver.recv().await.unwrap().unwrap();
        println!("{} kronor and {} ore", result.price.0, result.price.1);
        if let Some((kr, ore)) = result.member_price {
            println!("{} kronor and {} ore", kr, ore);
        }
        assert_eq!(result.id, 1);
        let result = receiver.recv().await.unwrap().unwrap();
        println!("{} kronor and {} ore", result.price.0, result.price.1);
        if let Some((kr, ore)) = result.member_price {
            println!("{} kronor and {} ore", kr, ore);
        }
        // should not panic
    }
}
