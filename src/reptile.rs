use crate::database::get_mysql_connection;
use crate::structs::BookChapter;
use crate::utils::get_text_from_response;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use log::error;
use reqwest::header;
use scraper::{ElementRef, Html, Node, Selector};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use tracing::info;

pub async fn parse_book_directory(book_str: &str, book_id: i64) -> Option<String> {
    let base_url = "http://www.qiqixs.info/";
    let base_url = format!("{}{}", base_url, book_str);
    let tail = "/?_=1772519989725";
    let http_client = reqwest::ClientBuilder::new()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36 Edg/136.0.0.0")
        .connect_timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    let res = http_client
        .get(&format!("{}{}", base_url, tail))
        .send()
        .await
        .unwrap();
    if res.status().as_u16() != 200 {
        error!("request failed: {}", res.status());
        return None;
    }
    info!("status: {}", res.status().as_str());
    let doc = get_text_from_response(res).await.unwrap();
    let book_title = get_title(&doc);
    info!("title -> {}", book_title);
    let mut tasks = FuturesUnordered::new();
    // 获取所有目录和目录的url
    let dl_selector = Selector::parse("div.list dl").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    for dl in doc.select(&dl_selector) {
        for (index, a_element) in dl.select(&a_selector).enumerate() {
            let href = a_element.value().attr("href").unwrap().to_string();
            let html = a_element.inner_html().to_string(); // clone 出纯 String
            let base_url = base_url.clone();
            let book_title = book_title.clone();
            let id = book_id;
            tasks.push(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                parse_book_content(&base_url, &href, &html, index, &book_title, id).await
            });
        }
    }

    let mut err_str = vec![];
    // await 所有任务
    while let Some(result) = tasks.next().await {
        if let Err(e) = result {
            err_str.push(e);
        }
    }
    error!("total failed [{}] -> [{:?}]", err_str.len(), &err_str);
    Some(book_title)
}

async fn parse_book_content(
    base_url: &str,
    tail: &str,
    article_title: &str,
    index: usize,
    book_title: &str,
    book_id: i64,
) -> Result<(), String> {
    let old_url = base_url;
    let base_url = format!("{}/{}", base_url, tail);
    let mut flag = false;
    for count in 0..20 {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("text/html"),
        );
        let http_client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(3))
            .default_headers(headers)
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

        let doc = get_text_from_response(rsp).await;
        if doc.is_err() {
            error!("parse rsp failed! {:?}", doc);
            continue;
        }
        let doc = doc.unwrap();
        let selector = Selector::parse(".content").unwrap();
        let vec = doc.select(&selector).collect::<Vec<_>>();
        let content = match vec.first() {
            None => {
                info!("{:?}", doc);
                error!(
                    "[{count}]content is null : [{}] failed --> url : [{}] tail : [{}]",
                    article_title, base_url, tail
                );
                continue;
            }
            Some(v) => v,
        };

        let mut str = String::new();
        for child in content.children() {
            match child.value() {
                // 如果是文本节点，加入正文
                Node::Text(t) => {
                    let line = t.trim();
                    if !line.is_empty() {
                        str.push_str(line);
                        str.push('\n');
                    }
                }
                // 如果是元素节点
                Node::Element(e) => {
                    // 忽略 <div class="con_show_l">
                    if e.name() == "div" {
                        if let Some(class) = e.attr("class") {
                            if class.contains("con_show_l") {
                                continue;
                            }
                        }
                    }
                    // 保留 <br> 为换行
                    if e.name() == "br" {
                        str.push('\n');
                    }
                }
                _ => {}
            }
        }

        if !Path::exists(Path::new(book_title)) {
            std::fs::create_dir_all(&book_title).unwrap();
        }
        let file_path = format!(
            "{}/{}.txt",
            book_title,
            index,
            // article_title.replace(" ", "_")
        );
        let pool = get_mysql_connection().await;
        let book_chapter = BookChapter {
            id: None,
            book_id,
            title: article_title.to_string(),
            chapter_index: index as i32,
            word_count: 0,
            file_path: Some(format!("/{}", file_path)),
            created_at: "".to_string(),
        };
        BookChapter::create_chapter(&pool, &book_chapter)
            .await
            .unwrap();

        std::fs::File::create(file_path)
            .unwrap()
            .write_all(str.as_bytes())
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
        Err(article_title.to_string())
    } else {
        Ok(())
    }
}

fn get_title(doc: &Html) -> String {
    let selector_title = Selector::parse(".title").unwrap();
    let selector_h1 = Selector::parse("h1").unwrap();
    let title_doc: Vec<_> = doc.select(&selector_title).map(|v| v).collect();
    if title_doc.is_empty() {
        error!("doc html -> {:?}", doc.html());
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
pub fn merge_book(book_title: &str) {
    let path = Path::new(book_title);
    let total_file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(format!("{}.txt", book_title))
        .unwrap();
    let mut writer = BufWriter::new(total_file);

    let mut file_vec: Vec<String> = vec![];
    let mut bytes = [0; 1024];
    match std::fs::read_dir(path) {
        Ok(dir) => dir.for_each(|v| match v {
            Ok(dir_entry) => {
                let file_name = dir_entry.file_name();
                let file_name = file_name.to_str().unwrap();
                file_vec.push(file_name.to_string());
            }
            Err(error) => {
                error!("get dir entry error -> {}", error);
            }
        }),
        Err(e) => {
            error!("merge book failed -> {}", e);
        }
    };

    file_vec.sort_by(|a, b| {
        let a_split: Vec<&str> = a.split("_").collect();
        let b_split: Vec<&str> = b.split("_").collect();
        let a_num = i32::from_str(a_split[0]).map_or_else(
            |e| {
                error!("parse num error -> {}", e);
                -1
            },
            |v| v,
        );
        let b_num = i32::from_str(b_split[0]).map_or_else(
            |e| {
                error!("parse num error -> {}", e);
                -1
            },
            |v| v,
        );
        a_num.cmp(&b_num)
    });
    file_vec.iter().for_each(|v| println!("{}", v));

    file_vec.iter().for_each(|path| {
        let raw = std::fs::File::open(format!("{}/{}", book_title, path))
            .expect(&format!("file [{}] open failed! ", path));
        let mut reader = BufReader::new(raw);
        loop {
            let n = reader.read(&mut bytes).unwrap();
            if n == 0 {
                info!("write file success -> {}", path);
                break;
            }
            writer.write_all(&bytes[..n]).unwrap();
        }
        writer.write(b"\n\n").unwrap();
    })
}
