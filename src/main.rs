use log::error;
use scraper::{Html, Selector};
use std::io::{BufReader, BufWriter, Error, Read, Write};
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

    let book_str = "195803";
    parse_book_directory(base_url, book_str).await;

    Ok(())
}

async fn parse_book_directory(base_url: &str, book_str: &str) -> Option<String> {
    let base_url = format!("{}{}", base_url, book_str);

    let http_client = reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0")
        .connect_timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let res = http_client.get(&base_url).send().await.unwrap();
    if res.status().as_u16() != 200 {
        error!("request failed: {}", res.status());
        return None;
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
        tokio::time::sleep(Duration::from_millis(500)).await;
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
    Some(book_title)
}

async fn parse_book_content(
    base_url: &str,
    tail: &str,
    article_title: String,
    index: usize,
    book_title: String,
) -> Result<(), String> {
    let old_url = base_url;
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
                *v = v.replace('\u{00a0}', "");
                *v = v.replace('\u{2003}', "");
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
            "overtime \"{}\",\"{}\",\"{}\".to_string(),{},\"{}\".to_string() ",
            old_url, tail, article_title, index, book_title
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
fn merge_book(book_title: &str) {
    let path = Path::new(book_title);
    let total_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{}.txt", book_title))
        .unwrap();
    let mut writer = BufWriter::new(total_file);
    let mut bytes = [0; 1024];
    match std::fs::read_dir(path) {
        Ok(dir) => dir.for_each(|v| {
            match v {
                Ok(dir_entry) => {
                    let raw = std::fs::File::open(dir_entry.path()).unwrap();
                    let mut reader = BufReader::new(raw);
                    loop {
                        let n = reader.read(&mut bytes).unwrap();
                        if n == 0 {
                            info!("write file success -> {}", dir_entry.path().display());
                            break;
                        }
                        writer.write_all(&bytes[..n]).unwrap();
                    }
                }
                Err(error) => {
                    error!("get dir entry error -> {}", error);
                }
            }
            writer.write(b"\n\n").unwrap();
        }),
        Err(e) => {
            error!("merge book failed -> {}", e);
        }
    };
}

#[cfg(test)]
mod tests {
    use crate::{merge_book, parse_book_content};
    use tracing::info;

    #[test]
    fn test_merge_book() {
        merge_book("乱世书");
    }

    #[tokio::test]
    async fn test_parse_book_content() {
        tracing_subscriber::fmt()
            .with_writer(std::io::stdout)
            .with_max_level(tracing::Level::INFO)
            .pretty()
            .init();
        info!("=========== initializing ========");

        loop {
            match parse_book_content(
                "http://www.qiqixs.info/195803",
                "66034609.html",
                "第二百六十章 剥茧抽丝".to_string(),
                267,
                "乱世书".to_string(),
            )
            .await
            {
                Ok(_) => {
                    break;
                }
                Err(_) => {}
            }
        }
    }
}
