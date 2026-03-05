use crate::book_intro::reptile_book_intro;
use log::info;
use regex::Regex;
use scraper::{Html, Selector};
use std::time::Duration;

pub async fn reptile_toplists() -> Result<(), String> {
    let http_client = reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0")
        .connect_timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let res = http_client
        .get("http://www.qiqixs.info/top/goodnum.html")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let html = res.text().await.map_err(|e| e.to_string())?;
    let document = Html::parse_document(&html);
    let book_link_vec = parse_toplists(document);
    for (book_name, book_num) in book_link_vec {
        info!("book_link: {} : {}", book_name, book_num);
        let book_id = reptile_book_intro(&book_num).await?;
        
    }
    Ok(())
}

fn parse_toplists(html: Html) -> Vec<(String, String)> {
    let dl_selector = Selector::parse("div.toplists dl").unwrap();
    let dd_selector = Selector::parse("dd").unwrap();
    let dt_selector = Selector::parse("dt").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let strong_selector = Selector::parse("strong").unwrap();

    let mut books = Vec::new();

    for dl in html.select(&dl_selector).skip(1) {
        // skip header dl
        let mut dd_iter = dl.select(&dd_selector);
        let rank = dd_iter
            .next()
            .map(|d| d.text().collect::<String>())
            .unwrap_or_default();
        let dt = dl.select(&dt_selector).next();
        let (title, link) = if let Some(dt) = dt {
            if let Some(a) = dt.select(&a_selector).next() {
                let title = a
                    .select(&strong_selector)
                    .next()
                    .map(|s| s.text().collect::<String>())
                    .unwrap_or_default();
                let link = a.value().attr("href").unwrap_or("").to_string();
                (title, link)
            } else {
                ("".to_string(), "".to_string())
            }
        } else {
            ("".to_string(), "".to_string())
        };
        let status = dd_iter
            .next()
            .map(|d| d.text().collect::<String>())
            .unwrap_or_default();
        let category = dd_iter
            .next()
            .map(|d| d.text().collect::<String>())
            .unwrap_or_default();
        let author = dd_iter
            .next()
            .and_then(|d| d.select(&a_selector).next())
            .map(|a| a.text().collect::<String>())
            .unwrap_or_default();
        let last_update = dd_iter
            .next()
            .map(|d| d.text().collect::<String>())
            .unwrap_or_default();

        let re = Regex::new(r"(\d+)\.html$").unwrap();

        if let Some(caps) = re.captures(&link) {
            let book_id = &caps[1];
            books.push((title, book_id.to_string()));
            println!("Book ID: {}", book_id); // 输出: 15785
        }
    }
    books
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_book_intro() {
        dotenv::dotenv().ok();
        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .with_max_level(tracing::Level::INFO)
            .pretty()
            .init();
        reptile_toplists().await.unwrap()
    }
}
