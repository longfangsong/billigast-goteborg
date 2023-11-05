#![feature(lazy_cell)]

use async_sqlite::{ClientBuilder, JournalMode};
use store::generate_crawl_tasks;
use store::save_crawl_results;
mod crawler;
mod model;
mod render;
mod store;

#[tokio::main]
async fn main() {
    let client = ClientBuilder::new()
        .path("./main.db")
        .journal_mode(JournalMode::Wal)
        .open()
        .await
        .unwrap();
    let tasks = generate_crawl_tasks(&client).await;
    let results = crawler::crawl(tasks).await;
    let results = results.into_iter().flatten().collect();
    save_crawl_results(&client, results).await;
    render::render_page(&client).await;
    render::render_plugin(&client).await;
}
