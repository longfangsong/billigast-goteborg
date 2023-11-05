use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Butik {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ButikPrice {
    pub rank: u16,
    pub price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commodity {
    pub id: u32,
    pub name: String,
    pub unit: String,
    pub butik_prices: Vec<Option<ButikPrice>>,
}
