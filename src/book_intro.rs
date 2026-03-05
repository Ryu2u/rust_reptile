use crate::database::get_mysql_connection;
use crate::structs::Book;
use log::{error, info};
use scraper::Html;
use std::str::FromStr;
use std::time::Duration;

pub async fn reptile_book_intro(book_num: &str) -> Result<i64, String> {
    let base_url = "http://www.qiqixs.info/";
    let http_client = reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0")
        .connect_timeout(Duration::from_secs(3))
        .build()
        .unwrap();

    let res = http_client
        .get(&format!("{}/book/{}.html", base_url, book_num))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let html = res.text().await.map_err(|e| e.to_string())?;

    let document = Html::parse_document(&html);

    parse_book_intro(&document).await
}

async fn parse_book_intro(document: &Html) -> Result<i64, String> {
    use log::info;
    use regex::Regex;
    use scraper::{ElementRef, Selector};

    // 书名
    let name = document
        .select(&Selector::parse("div.title h1").unwrap())
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default()
        .trim()
        .to_string();

    // 作者
    let author = document
        .select(&Selector::parse("#author").unwrap())
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default()
        .replace("作者：", "")
        .replace("/", "")
        .trim()
        .to_string();

    // 状态
    let status = document
        .select(&Selector::parse("div.fullflag").unwrap())
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default()
        .replace("状态：", "")
        .trim()
        .to_string();
    let status = match status.as_str() {
        "连载中" => 0,
        "已完成" => 1,
        _ => 0,
    };

    // 统计信息
    let stat_text = document
        .select(&Selector::parse("div.info > p").unwrap())
        .next()
        .map(|e| e.text().collect::<String>())
        .unwrap_or_default();

    let stat_re =
        Regex::new(r"阅读：(\d+).*?收藏：(\d+).*?推荐：(\d+).*?字数：(\d+).*?更新：([\d\-:\s]+)")
            .unwrap();

    let (read_count, collect_count, recommend_count, word_count, update_time) =
        if let Some(c) = stat_re.captures(&stat_text) {
            (
                c[1].to_string(),
                c[2].to_string(),
                c[3].to_string(),
                c[4].to_string(),
                c[5].to_string(),
            )
        } else {
            ("".into(), "".into(), "".into(), "".into(), "".into())
        };

    // 封面
    let cover = document
        .select(&Selector::parse("div.cover img").unwrap())
        .next()
        .and_then(|e| e.value().attr("src"))
        .unwrap_or("")
        .to_string();

    // 简介
    let intro = document
        .select(&Selector::parse(".xiaoshuo .info-text").unwrap())
        .next()
        .map(|element| {
            // 过滤掉 tags 和 same_author
            element
                .children()
                .filter_map(|child| {
                    if let Some(el) = ElementRef::wrap(child) {
                        let id = el.value().attr("id");
                        if id == Some("tags") || id == Some("same_author") {
                            return None;
                        }
                        Some(el.text().collect::<String>())
                    } else {
                        Some(child.value().as_text().map_or("", |v| v).to_string())
                    }
                })
                .collect::<String>()
                .replace("&nbsp;", "")
                .trim()
                .to_string()
        })
        .unwrap_or_default();

    info!("书名: {}", name);
    info!("作者: {}", author);
    info!("状态: {}", status);
    info!("阅读量: {}", read_count);
    info!("收藏量: {}", collect_count);
    info!("推荐量: {}", recommend_count);
    info!("字数: {}", word_count);
    info!("更新时间: {}", update_time);
    info!("封面: {}", cover);
    info!("简介: {}", intro);

    let pool = get_mysql_connection().await;
    let book = Book {
        id: None,
        name: name.clone(),
        author: author.to_string(),
        cover_url: Some(cover.to_string()),
        path_url: Some("".to_string()),
        description: Some(intro.to_string()),
        category_id: None,
        word_count: i32::from_str(&word_count).map_or_else(|_| 0, |v| v),
        chapter_count: 0,
        status,
        view_count: i64::from_str(&read_count).map_or_else(|_| 0, |v| v),
        price: 200,
        is_deleted: 0,
        created_at: update_time.to_string(),
        updated_at: update_time.to_string(),
    };
    match Book::create_book(&pool, &book).await {
        Ok(id) => Ok(id as i64),
        Err(e) => {
            error!("添加书籍[{}]失败: {}", name, e);
            Err(format!("{}", e))
        }
    }
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
        let book_num = "15785";
        reptile_book_intro(book_num).await.unwrap();
    }
}
