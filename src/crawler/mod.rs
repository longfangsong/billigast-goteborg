use anyhow::Result;
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use indicatif::ProgressBar;
use rand::Rng;
use std::time::Duration;
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::wrappers::ReceiverStream;

mod coop;
mod hemkop;
mod ica;
mod willys;

pub fn parse_unit_price(unit_price: &str) -> Option<Decimal> {
    let parts: Vec<&str> = unit_price
        .trim_end_matches("kr")
        .trim()
        .split(|c| c == ',' || c == '.' || c == ':')
        .collect();
    let kronor = parts[0].parse::<u16>().ok()?;
    let ore = parts.get(1).unwrap_or(&"0").parse::<u8>().unwrap_or(0);
    Some((kronor, ore))
}

#[derive(Debug, Clone)]
pub struct CrawlTask {
    pub id: u64,
    pub butik_id: Option<String>,
    pub butik_name: String,
    pub fetch_id: String,
}

pub type Decimal = (u16, u8);

#[derive(Debug, Clone)]
pub struct CrawlResult {
    pub id: u64,
    pub price: Decimal,
    pub member_price: Option<Decimal>,
}

#[async_trait]
pub trait StoreCrawler: Clone + Send + 'static {
    // fixme: error handling
    async fn crawl_price(&self, task: CrawlTask) -> Result<CrawlResult>;

    fn crawl_delay() -> u64 {
        let mut rng = rand::thread_rng();
        rng.gen_range(1..5)
    }

    async fn start_crawler(
        &self,
        mut input_channel: mpsc::Receiver<CrawlTask>,
    ) -> mpsc::Receiver<Result<CrawlResult>> {
        let (tx, rx) = mpsc::channel(32);
        let this = self.clone();
        tokio::spawn(async move {
            while let Some(task) = input_channel.recv().await {
                let result = this.crawl_price(task).await;
                let random = Self::crawl_delay();
                sleep(Duration::from_secs(random)).await;
                tx.send(result).await.unwrap();
            }
        });
        rx
    }
}

pub fn with_inc_progressbar(
    progress_bar: ProgressBar,
    receiver: ReceiverStream<Result<CrawlResult>>,
) -> impl Stream<Item = Result<CrawlResult>> {
    receiver.map(move |it| {
        progress_bar.inc(1);
        it
    })
}

pub async fn crawl(tasks: Vec<CrawlTask>) -> Vec<Result<CrawlResult>> {
    let (tx_coop, rx_coop) = mpsc::channel(32);
    let (tx_hemkop, rx_hemkop) = mpsc::channel(32);
    let (tx_ica, rx_ica) = mpsc::channel(32);
    let (tx_willys, rx_willys) = mpsc::channel(32);

    let pb = ProgressBar::new(tasks.len() as u64);
    let coop = with_inc_progressbar(
        pb.clone(),
        ReceiverStream::new(coop::CoopCrawler.start_crawler(rx_coop).await),
    );
    let hemkop = with_inc_progressbar(
        pb.clone(),
        ReceiverStream::new(hemkop::HemkopCrawler.start_crawler(rx_hemkop).await),
    );
    let ica = with_inc_progressbar(
        pb.clone(),
        ReceiverStream::new(ica::ICACrawler.start_crawler(rx_ica).await),
    );
    let willys = with_inc_progressbar(
        pb.clone(),
        ReceiverStream::new(willys::WillysCrawler.start_crawler(rx_willys).await),
    );

    tokio::spawn(async move {
        for task in tasks {
            if task.butik_name.to_lowercase().contains("coop") {
                tx_coop.send(task).await.unwrap();
            } else if task.butik_name.to_lowercase().contains("hemköp") {
                tx_hemkop.send(task).await.unwrap();
            } else if task.butik_name.to_lowercase().contains("ica") {
                tx_ica.send(task).await.unwrap();
            } else if task.butik_name.to_lowercase().contains("willys") {
                tx_willys.send(task).await.unwrap();
            } else {
                panic!("Unknown how to crawl for {}", task.butik_name)
            }
        }
    });
    let merged = futures::stream::select_all(vec![coop, hemkop, ica, willys]);
    let result = merged.collect().await;
    pb.finish_with_message("done");
    result
}
