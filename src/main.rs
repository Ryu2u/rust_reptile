mod book_category;
mod book_intro;
mod database;
mod reptile;
mod structs;
mod toplist;
mod utils;

use crate::database::get_mysql_connection;
use crate::reptile::parse_book_directory;
use crate::structs::{Book, RankType};
use crate::toplist::reptile_toplists;
use std::io::Error;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_writer(std::io::stdout)
        .with_max_level(tracing::Level::INFO)
        .pretty()
        .init();
    info!("=========== initializing ========");
    let pool = get_mysql_connection().await;
    parse_book_directory("19444", 1, &pool).await?;
    // reptile_toplists("weekvisit", RankType::HotSales).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::reptile::merge_book;

    #[test]
    fn test_merge_book() {
        merge_book("借剑");
    }
}
