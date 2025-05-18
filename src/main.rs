use log::error;
use scraper::{Html, Selector};
use std::io::{Error, Write};
use std::path::Path;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_max_level(tracing::Level::INFO)
        .pretty()
        .init();
    info!("=========== initializing ========");
    let base_url = "http://www.qiqixs.info/";

    let book_str = "201698";
    parse_book_directory(base_url, book_str).await;

    Ok(())
}

async fn parse_book_directory(base_url: &str, book_str: &str) {
    let base_url = format!("{}/{}", base_url, book_str);

    let res = reqwest::get(&base_url).await.unwrap();
    let res = res.text().await.unwrap();
    let doc = Html::parse_document(&res);
    let book_title = get_title(&doc);
    println!("{}", book_title);
    let selector = Selector::parse("dl").unwrap();
    let html_vec: Vec<_> = doc.select(&selector).map(|v| v).collect();
    let dl = html_vec.first().unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let a_vec: Vec<_> = dl.select(&a_selector).collect();
    let mut handle = vec![];
    for (index, a_element) in a_vec.iter().enumerate() {
        let href = a_element.value().attr("href").unwrap().to_string();
        info!("{} - {}", a_element.inner_html(), href);
        let html = a_element.inner_html().clone();
        let base_url = base_url.clone();
        let book_title = book_title.clone();
        handle.push(tokio::spawn(async move {
            parse_book_content(&base_url, &href, html, index, book_title).await;
        }));
    }
    for t in handle {
        t.await.unwrap();
    }
}

async fn parse_book_content(
    base_url: &str,
    tail: &str,
    article_title: String,
    index: usize,
    book_title: String,
) -> Option<()> {
    let base_url = format!("{}/{}", base_url, tail);
    let mut res = String::new();
    let mut count = 0;
    while res.trim().is_empty() && count < 3 {
        let rsp = reqwest::get(&base_url).await.unwrap();
        res = rsp.text().await.unwrap();
        count += 1;
    }
    if count >= 3 {
        error!(
            "download over 3 times, url : [{}] tail : [{}]",
            base_url, tail
        );
    }
    let doc = Html::parse_document(&res);
    let selector = Selector::parse(".content").unwrap();
    let vec: Vec<_> = doc.select(&selector).collect();
    let content = match vec.first() {
        None => {
            error!(
                "download title : [{}] failed --> url : [{}] tail : [{}]",
                article_title, base_url, tail
            );
            return None;
        }
        Some(v) => v,
    };
    let str = content.text().collect::<Vec<&str>>();
    let mut str: Vec<String> = str.iter().map(|v| v.to_string()).collect();
    str.iter_mut().for_each(|v| {
        if !v.trim().is_empty() {
            *v = v.replace(" ", "");
        }
    });
    str.remove(0);
    str.insert(0, format!("{}\n\n", article_title));
    if !Path::exists(Path::new(book_title.as_str())) {
        std::fs::create_dir_all(&book_title).unwrap();
    }
    std::fs::File::create(format!(
        "{}/{}_{}.txt",
        book_title,
        index,
        article_title.replace(" ", "_")
    ))
    .unwrap()
    .write_all(str.join("  ").as_bytes())
    .unwrap();
    Some(())
}

fn get_title(doc: &Html) -> String {
    let selector_title = Selector::parse(".title").unwrap();
    let selector_h1 = Selector::parse("h1").unwrap();
    let title_doc: Vec<_> = doc.select(&selector_title).map(|v| v).collect();
    let title_element = title_doc.first().unwrap();
    let res: Vec<_> = title_element.select(&selector_h1).map(|v| v).collect();
    res.first().unwrap().inner_html().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_book_content() {
        let base_url = "http://www.qiqixs.info/195803/";
        let tail = "";
        let title = "".to_string();
        let book_title = "195803".to_string();
        let res = parse_book_content(base_url, tail, title, 1, book_title).await;
        res.unwrap();
    }

    #[tokio::test]
    async fn test_book_title() {
        let base_url = "http://www.qiqixs.info/";

        let book_str = "201698";
        parse_book_directory(base_url, book_str).await;
    }
}
