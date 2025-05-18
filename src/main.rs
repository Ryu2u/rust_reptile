use scraper::{Html, Selector};
use std::io::{Error, Write};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_max_level(tracing::Level::INFO)
        .pretty()
        .init();
    info!("=========== initializing ========");
    let base_url = "http://www.qiqixs.info";

    parse_book_directory(base_url).await;

    Ok(())
}

async fn parse_book_directory(base_url: &str) {
    let base_url = format!("{}{}", base_url, "/27563");

    let res = reqwest::get(&base_url).await.unwrap();
    let res = res.text().await.unwrap();
    let doc = Html::parse_document(&res);
    let selector = Selector::parse("dl").unwrap();
    info!("{:?}", selector);
    let res = doc.select(&selector);
    let html_vec: Vec<_> = res.map(|v| v).collect();
    info!("{:?}", html_vec);
    let dl = html_vec.first().unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let a_vec: Vec<_> = dl.select(&a_selector).collect();
    for a_element in a_vec {
        info!("{:?}", a_element);
        info!("{}", a_element.inner_html());
        let href = a_element.value().attr("href").unwrap();
        info!("{}", href);
        parse_book_content(&base_url, href, a_element.inner_html().clone()).await;
    }
}

async fn parse_book_content(base_url: &str, tail: &str, title: String) {
    let base_url = format!("{}/{}", base_url, tail);
    let res = reqwest::get(&base_url).await.unwrap();
    let res = res.text().await.unwrap();
    let doc = Html::parse_document(&res);
    let selector = Selector::parse(".content").unwrap();
    let vec: Vec<_> = doc.select(&selector).collect();
    info!("{:?}", vec);
    let content = vec.first().unwrap();
    let mut str = content.text().collect::<Vec<&str>>();
    let mut str: Vec<String> = str.iter().map(|v| v.to_string()).collect();
    str.iter_mut().for_each(|v| {
        *v = v.replace(" ","");
        v.insert_str(0,"  ");
    });
    str.remove(0);
    str.insert(0, title.to_string());
    str.iter().for_each(|v| info!("{}", v));
    std::fs::File::create(format!("dir/{}.txt", title.replace(" ", "_")))
        .unwrap()
        .write_all(str.join("\n").as_bytes())
        .unwrap();
}
