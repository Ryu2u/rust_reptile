use crate::database::get_mysql_connection;
use crate::structs::Book;
use crate::utils::get_text_from_response;
use log::error;
use regex::Regex;
use scraper::{ElementRef, Selector};
use std::time::Duration;
use chrono::NaiveDateTime;
use tracing::info;

/// http://www.qiqixs.info/xuanhuan/
pub async fn reptile_category(base_url: &str, category_url: &str) -> Option<()> {
    let http_client = reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0")
        .connect_timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let res = http_client
        .get(&format!("{}{}", base_url, category_url))
        .send()
        .await
        .unwrap();
    if res.status().as_u16() != 200 {
        error!("request failed: {}", res.status());
        return None;
    }
    let doc = match get_text_from_response(res).await {
        Ok(x) => x,
        Err(e) => {
            error!("{}", e);
            return None;
        }
    };
    // 获取
    let book_class = Selector::parse(".book").unwrap();
    let book_vec: Vec<ElementRef> = doc.select(&book_class).map(|v| v).collect();
    for v in book_vec {
        info!("{:?}", v.inner_html());
        parse_book_info(&v.inner_html(), 1).await;
    }
    Some(())
}
async fn parse_book_info(html: &str, category_id: i64) {
    let cover_re = Regex::new(r#"<img[^>]*src="([^"]+)""#).unwrap();
    let cover = cover_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    info!("封面: {}", cover);

    let name_re = Regex::new(r#"<h2>\s*<a[^>]*>(.*?)</a>"#).unwrap();

    let raw_name = name_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    let book_name = raw_name
        .replace("《", "")
        .replace("》", "")
        .replace("最新章节", "")
        .trim()
        .to_string();

    info!("书名: {}", book_name);

    // 2️⃣ 作者
    let author_re = Regex::new(r#"作者：<a[^>]*>([^<]+)</a>"#).unwrap();
    let author = author_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    info!("作者: {}", author);

    // 3️⃣ 更新状态
    let status_re = Regex::new(r#"<span[^>]*>([^<]+)</span>"#).unwrap();
    let status = status_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    info!("状态: {}", status);

    let status = match status {
        "连载中" => 0,
        "已完成" => 1,
        _ => 0,
    };

    // 4️⃣ 更新时间
    let time_re = Regex::new(r#"更新时间：([0-9\-]+)"#).unwrap();
    let update_time = time_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    info!("更新时间: {}", update_time);

    // 5️⃣ 简介（第一个 <p> 标签）
    let intro_re = Regex::new(r#"<p>(.*?)</p>"#).unwrap();
    let intro = intro_re
        .captures(html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");

    info!("简介: {}", intro);

    let pool = get_mysql_connection().await;
    let book = Book {
        id: None,
        name: book_name.clone(),
        author: author.to_string(),
        cover_url: Some(cover.to_string()),
        path_url: Some("".to_string()),
        description: Some(intro.to_string()),
        category_id: Some(category_id),
        word_count: 0,
        chapter_count: 0,
        status: status,
        view_count: 0,
        price: 200,
        is_deleted: 0,
        created_at: NaiveDateTime::default(),
        updated_at: NaiveDateTime::default(),
    };

    match Book::create_book(&pool, &book).await {
        Ok(_) => {}
        Err(e) => {
            error!("更新书籍[{}]失败: {}", book_name, e);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_reptile_category() {
        dotenv::dotenv().ok();
        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .with_max_level(tracing::Level::INFO)
            .pretty()
            .init();
        let base_url = "http://www.qiqixs.info/";
        let category_name = "xuanhuan";
        reptile_category(base_url, category_name).await;
    }
}
