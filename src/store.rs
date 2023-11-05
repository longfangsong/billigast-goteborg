use std::iter;

use async_sqlite::{rusqlite::params, Client};
use itertools::Itertools;

use crate::{
    crawler::{CrawlResult, CrawlTask},
    model::{Butik, ButikPrice, Commodity},
};

pub async fn generate_crawl_tasks(client: &Client) -> Vec<CrawlTask> {
    client
        .conn(|connection| {
            let mut stmt = connection
                .prepare(
                    "SELECT ButikCommodity.id as id, 
                            Butik.site_id as butik_id,
                            Butik.name as butik_name,
                            ButikCommodity.fetch_id as fetch_id
                        FROM ButikCommodity, Butik
                        WHERE Butik.id = ButikCommodity.butik_id;",
                )
                .unwrap();
            let mut rows = stmt.query([]).unwrap();
            let mut tasks = Vec::new();
            while let Some(row) = rows.next().unwrap() {
                let id: u64 = row.get(0).unwrap();
                let butik_id: Option<String> = row.get(1).unwrap();
                let butik_name: String = row.get(2).unwrap();
                let fetch_id: String = row.get(3).unwrap();
                tasks.push(CrawlTask {
                    id,
                    butik_id,
                    butik_name,
                    fetch_id,
                });
            }
            Ok(tasks)
        })
        .await
        .unwrap()
}

pub async fn save_crawl_results(client: &Client, results: Vec<CrawlResult>) {
    client
        .conn(|connection| {
            let mut stmt = connection
                .prepare_cached("delete from ButikCommodityRecord;")
                .unwrap();
            stmt.execute([]).unwrap();
            let mut no_member_price_stmt = connection
                .prepare_cached("INSERT INTO ButikCommodityRecord(price, id) VALUES (?1, ?2);")
                .unwrap();
            let mut member_price_stmt = connection
                .prepare_cached(
                    "INSERT INTO ButikCommodityRecord(price, member_price, id) VALUES (?1, ?2, ?3);",
                )
                .unwrap();
            for result in results {
                if let Some(member_price) = result.member_price {
                    member_price_stmt
                        .execute(params![
                            format!("{}.{:02}", result.price.0, result.price.1),
                            format!("{}.{:02}", member_price.0, member_price.1),
                            result.id,
                        ])
                        .unwrap();
                } else {
                    no_member_price_stmt
                        .execute(params![
                            format!("{}.{:02}", result.price.0, result.price.1),
                            result.id,
                        ])
                        .unwrap();
                }
            }
            Ok(())
        })
        .await
        .unwrap();
}

pub async fn get_butiks(client: &Client) -> Vec<Butik> {
    client
        .conn(|connection| {
            let mut stmt = connection
                .prepare("SELECT id, name FROM Butik ORDER BY id;")
                .unwrap();
            Ok(stmt
                .query_map([], |row| {
                    Ok(Butik {
                        id: row.get(0).unwrap(),
                        name: row.get(1).unwrap(),
                    })
                })
                .unwrap()
                .map(Result::unwrap)
                .collect())
        })
        .await
        .unwrap()
}

pub async fn get_commodities(client: &Client) -> Vec<Commodity> {
    client
        .conn(|connection| {
            let mut stmt = connection
                .prepare("SELECT id, name, unit FROM Commodity ORDER BY id;")
                .unwrap();
            let mut commodities = stmt
                .query_map([], |row| {
                    Ok(
                        Commodity {
                            id: row.get(0).unwrap(),
                            name: row.get(1).unwrap(),
                            unit: row.get(2).unwrap(),
                            // fixme: use something else than `6`
                            butik_prices: iter::repeat(None).take(6).collect(),
                        },
                    )
                })
                .unwrap()
                .map(Result::unwrap)
                .collect_vec();
            let mut ranked = connection
                .prepare("
WITH CheapestInButik AS (
    WITH YourPriceRank AS (
        WITH YourPrice AS (SELECT ButikCommodity.id, IIF(
            butik_id IN (1,2,3,4,5,6),
            IFNULL(
                ButikCommodityRecord.member_price, 
                ButikCommodityRecord.price
            ),
            ButikCommodityRecord.price
            ) AS price_for_you, butik_id, commodity_id, in_stock
            FROM ButikCommodity, ButikCommodityRecord
            WHERE ButikCommodity.id = ButikCommodityRecord.id
        )
        SELECT *, RANK() OVER (
                PARTITION BY butik_id, commodity_id
                ORDER BY price_for_you
        ) AS inner_rank
        FROM YourPrice
    )
    SELECT id, price_for_you, butik_id, commodity_id from YourPriceRank
    WHERE inner_rank = 1
)
SELECT butik_id, commodity_id, price_for_you, RANK() OVER (PARTITION BY commodity_id ORDER BY price_for_you ASC) as rank
FROM CheapestInButik;",
                )
                .unwrap();
            let mut rows = ranked.query([])?;
            while let Ok(Some(row)) = rows.next() {
                let butik_id = row.get::<_, u32>(0).unwrap();
                let commodity_id = row.get::<_, u32>(1).unwrap();
                let price = row.get::<_, f64>(2).unwrap();
                let rank = row.get::<_, u32>(3).unwrap();
                commodities[commodity_id as usize - 1].butik_prices[butik_id as usize - 1] =
                    Some(ButikPrice { rank: rank as _, price });
            }
            Ok(commodities)
        })
        .await
        .unwrap()
}

#[cfg(test)]
mod tests {
    use async_sqlite::{ClientBuilder, JournalMode};

    use super::*;
    #[tokio::test]
    async fn test_generate_crawl_tasks() {
        let client = ClientBuilder::new()
            .path("./main.db")
            .journal_mode(JournalMode::Wal)
            .open()
            .await
            .unwrap();
        let tasks = generate_crawl_tasks(&client).await;
        dbg!(tasks);
    }
}
