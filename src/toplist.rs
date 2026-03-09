use crate::book_intro::reptile_book_intro;
use crate::database::get_mysql_connection;
use crate::structs::{BookRanking, RankType};
use chrono::Local;
use log::info;
use regex::Regex;
use reqwest::header;
use scraper::{Html, Selector};
use std::str::FromStr;
use crate::reptile::parse_book_directory;

struct TopList {
    book_name: String,
    book_num: String,
    rank: String,
    rank_type: RankType,
}

pub async fn reptile_toplists(toplist_type: &str, rank_type: RankType) -> anyhow::Result<()> {
    let mut headers = header::HeaderMap::new();
    headers.insert(header::ACCEPT, header::HeaderValue::from_static("*/*"));
    headers.insert(
        "master-key",
        header::HeaderValue::from_static("8102b22a5e81e840176d9f381ec6f837"),
    );
    let http_client = reqwest::ClientBuilder::new()
        .user_agent("User-Agent: PostmanRuntime-ApipostRuntime/1.1.0")
        .default_headers(headers)
        .build()?;
    let res = http_client
        .get(format!("http://www.qiqixs.info/top/{}.html", toplist_type))
        .send()
        .await?;

    let html = res.text().await?;
    let pool = get_mysql_connection().await;
    info!("========== reptile ========");
    let document = Html::parse_document(&html);
    let book_link_vec = parse_toplists(document, rank_type);
    for item in book_link_vec {
        info!("book_link: {} : {}", item.book_name, item.book_num);
        let book_id = reptile_book_intro(&pool, &item.book_num).await?;
        let period = "total";
        let book_ranking = BookRanking {
            id: 0,
            book_id,
            rank_type: rank_type.as_str().to_string(),
            rank: i32::from_str(&item.rank).unwrap(),
            score: 0,
            extra_data: None,
            period: Some(period.to_string()),
            stat_date: Some(Local::now().date_naive()),
            created_at: Local::now().naive_local(),
            updated_at: Local::now().naive_local(),
        };

        match BookRanking::exists(
            &pool,
            &book_ranking.rank_type,
            book_ranking.book_id,
            &period,
            book_ranking.stat_date,
        )
        .await
        {
            Ok(true) => {
                info!("book {} is exists", item.book_name);
                continue;
            }
            _ => {}
        };

        BookRanking::insert_ranking(&pool, &book_ranking)
            .await
            .unwrap();
        parse_book_directory(&item.book_num, book_id,&pool).await.unwrap();
    }
    Ok(())
}

fn parse_toplists(html: Html, rank_type: RankType) -> Vec<TopList> {
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

        let re = Regex::new(r"(\d+)\.html$").unwrap();
        if let Some(caps) = re.captures(&link) {
            let book_id = &caps[1];

            books.push(TopList {
                book_name: title,
                book_num: book_id.to_string(),
                rank,
                rank_type,
            });
            println!("Book ID: {}", book_id); // 输出: 15785
        }
    }
    books
}

#[cfg(test)]
mod test {
    

    #[tokio::test]
    async fn test_book_intro() {
        dotenv::dotenv().ok();
        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .with_max_level(tracing::Level::INFO)
            .pretty()
            .init();
    }
}
