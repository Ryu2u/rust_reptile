use log::error;
use scraper::{Html, Selector};
use std::io::{Error, Write};
use std::path::Path;
use std::time::Duration;
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

    let book_str = "218556";
    parse_book_directory(base_url, book_str).await;

    Ok(())
}

async fn parse_book_directory(base_url: &str, book_str: &str) {
    let base_url = format!("{}{}", base_url, book_str);

    let http_client = reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0")
        .connect_timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let res = http_client.get(&base_url).send().await.unwrap();
    if res.status().as_u16() != 200 {
        error!("request failed: {}", res.status());
        return;
    }
    info!("status: {}", res.status().as_str());
    let res = res.text().await.unwrap();
    let doc = Html::parse_document(&res);
    let book_title = get_title(&doc);
    info!("title -> {}", book_title);
    let selector = Selector::parse("dl").unwrap();
    let html_vec: Vec<_> = doc.select(&selector).map(|v| v).collect();
    let dl = html_vec.first().unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let a_vec: Vec<_> = dl.select(&a_selector).collect();
    let mut handle = vec![];
    for (index, a_element) in a_vec.iter().enumerate() {
        let href = a_element.value().attr("href").unwrap().to_string();
        let html = a_element.inner_html().clone();
        let base_url = base_url.clone();
        let book_title = book_title.clone();
        tokio::time::sleep(Duration::from_millis(100)).await;
        handle.push(tokio::spawn(async move {
            parse_book_content(&base_url, &href, html, index, book_title).await
        }));
    }

    let mut err_str = vec![];
    for t in handle {
        if let Err(v) = t.await.unwrap() {
            err_str.push(v);
        }
    }
    error!("total failed [{}] -> [{:?}]", err_str.len(), &err_str);
}

async fn parse_book_content(
    base_url: &str,
    tail: &str,
    article_title: String,
    index: usize,
    book_title: String,
) -> Result<(), String> {
    let base_url = format!("{}/{}", base_url, tail);
    let mut flag = false;
    for count in 0..20 {
        let http_client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(3))
            .build()
            .unwrap();
        let res_rsp = http_client.get(&base_url).send().await;
        if let Err(e) = res_rsp {
            error!("[{count}]fetch [{}] failed error -> {}", article_title, e);
            continue;
        }
        let rsp = res_rsp.unwrap();

        if rsp.status().as_u16() != 200 {
            error!("[{count}]fetch status is {}", rsp.status().as_str());
            continue;
        }

        let res = rsp.text().await.unwrap();
        let doc = Html::parse_document(&res);
        let selector = Selector::parse(".content").unwrap();
        let vec = doc.select(&selector).collect::<Vec<_>>();
        let content = match vec.first() {
            None => {
                error!(
                    "[{count}]content is null : [{}] failed --> url : [{}] tail : [{}]",
                    article_title, base_url, tail
                );
                continue;
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
        info!("{} - {}", article_title, base_url);
        flag = true;
        break;
    }
    if !flag {
        error!(
            "overtime [{}] failed --> url : [{}] tail : [{}]",
            article_title, base_url, tail
        );
        Err(article_title)
    } else {
        Ok(())
    }
}

fn get_title(doc: &Html) -> String {
    let selector_title = Selector::parse(".title").unwrap();
    let selector_h1 = Selector::parse("h1").unwrap();
    let title_doc: Vec<_> = doc.select(&selector_title).map(|v| v).collect();
    if title_doc.is_empty() {
        error!("doc -> {:?}", doc.errors);
        panic!("title document is empty");
    } else {
        info!("doc err -> {:?}", doc.errors);
    }
    let title_element = title_doc.first().unwrap();
    let res: Vec<_> = title_element.select(&selector_h1).map(|v| v).collect();
    res.first().unwrap().inner_html().to_string()
}

#[allow(unused)]
async fn merge_book(_dir_path: &str) {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
}
