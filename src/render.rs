use std::sync::LazyLock;

use async_sqlite::Client;
use tera::Tera;

use crate::store::{get_butiks, get_commodities};

static TERA: LazyLock<Tera> = LazyLock::new(|| Tera::new("template/*.template").unwrap());

pub async fn render_page(client: &Client) {
    let mut context = tera::Context::new();
    let butiks = get_butiks(client).await;
    context.insert("butiks", &butiks);
    let commodities = get_commodities(client).await;
    context.insert("commodities", &commodities);
    let rendered = TERA.render("index.html.template", &context).unwrap();
    std::fs::create_dir_all("./public").unwrap();
    std::fs::write("./public/index.html", rendered).unwrap();
}

pub async fn render_plugin(client: &Client) {
    let tera = match Tera::new("template/*") {
        Ok(t) => t,
        Err(e) => {
            panic!("Parsing error(s): {}", e);
        }
    };
    let mut context = tera::Context::new();
    let butiks = get_butiks(client).await;
    context.insert("butiks", &butiks);
    let commodities = get_commodities(client).await;
    context.insert("commodities", &commodities);
    let rendered = tera.render("ica.js.template", &context).unwrap();
    std::fs::write("./firefox-plugin/ica.js", rendered).unwrap();
    let rendered = tera.render("willys.js.template", &context).unwrap();
    std::fs::write("./firefox-plugin/willys.js", rendered).unwrap();
    let rendered = tera.render("coop.js.template", &context).unwrap();
    std::fs::write("./firefox-plugin/coop.js", rendered).unwrap();
}
