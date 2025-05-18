use log::error;
use scraper::{Html, Selector};
use std::io::{Error, Write};
use tokio::task::JoinHandle;
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

    parse_book_directory(base_url).await;

    Ok(())
}

async fn parse_book_directory(base_url: &str) {
    let base_url = format!("{}{}", base_url, "/195803");

    let res = reqwest::get(&base_url).await.unwrap();
    let res = res.text().await.unwrap();
    let doc = Html::parse_document(&res);
    let selector = Selector::parse("dl").unwrap();
    let res = doc.select(&selector);
    let html_vec: Vec<_> = res.map(|v| v).collect();
    let dl = html_vec.first().unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let a_vec: Vec<_> = dl.select(&a_selector).collect();
    let mut handle = vec![];
    for a_element in a_vec {
        let href = a_element.value().attr("href").unwrap();
        info!("{} - {}", a_element.inner_html(), href);
        handle.push(parse_book_content(&base_url, href, a_element.inner_html().clone()).await);
    }
    for t in handle {
        if let None = t {
        } else if let Some(v) = t {
            v.await.unwrap();
        }
    }
}

async fn parse_book_content(base_url: &str, tail: &str, title: String) -> Option<JoinHandle<()>> {
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
                title, base_url, tail
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
    str.insert(0, format!("{}\n\n", title));
    Some(tokio::spawn(async move {
        std::fs::File::create(format!("dir/{}.txt", title.replace(" ", "_")))
            .unwrap()
            .write_all(str.join("  ").as_bytes())
            .unwrap();
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_book_content() {
        let base_url = "http://www.qiqixs.info/195803/";
        let res = parse_book_content(base_url, "", "".to_string()).await;
        res.unwrap().await.unwrap();
    }
}
