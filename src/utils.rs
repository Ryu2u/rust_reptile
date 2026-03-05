use reqwest::Response;
use scraper::Html;
use std::string::FromUtf8Error;

pub async fn get_text_from_response(res: Response) -> Result<Html, FromUtf8Error> {
    let bytes = res.bytes().await.unwrap();
    match String::from_utf8(bytes.to_vec()) {
        Ok(x) => Ok(Html::parse_document(&x)),
        Err(e) => Err(e),
    }
}
