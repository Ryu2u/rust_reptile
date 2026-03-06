use anyhow::anyhow;
use futures::StreamExt;
use reqwest::{ClientBuilder, Response};
use scraper::Html;
use std::io::Write;
use std::string::FromUtf8Error;

pub async fn get_text_from_response(res: Response) -> Result<Html, FromUtf8Error> {
    let bytes = res.bytes().await.unwrap();
    match String::from_utf8(bytes.to_vec()) {
        Ok(x) => Ok(Html::parse_document(&x)),
        Err(e) => Err(e),
    }
}

/// download file
pub async fn download_img(img_url: &str) -> anyhow::Result<()> {
    let http_client = ClientBuilder::new().build()?;
    let rsp = http_client.get(img_url).send().await?;
    let total = rsp.content_length().unwrap_or(0);
    let progress = 0;
    let mut stream = rsp.bytes_stream();
    let file_name_reg = regex::Regex::new(r#"/([^/?#]+)(?:\?|#|$)"#)?;
    let file_name = match file_name_reg.captures(img_url) {
        None => {
            return Err(anyhow!("can't get file name from url[{}]", img_url));
        }
        Some(e) => e[1].to_string(),
    };
    let mut file = std::fs::File::create(file_name)?;
    while let Some(bytes) = stream.next().await {
        file.write_all(&bytes?)?;
        println!("progress: {}/{}", progress, total);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_download_img() {
        download_img("http://img.qiqixs.info/34/34211/34211s.jpg")
            .await
            .unwrap();
    }
}
