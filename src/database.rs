use sqlx::pool::PoolOptions;
use sqlx::{MySql, Pool};
use std::time::Duration;

pub async fn get_mysql_connection() -> Pool<MySql> {
    let url = dotenv::var("DATABASE_URL").expect("can't get MySql url from .env");
    let pool: Pool<MySql> = PoolOptions::new()
        // 最大连接数
        .max_connections(1000)
        // 连接池超时时间
        .acquire_timeout(Duration::from_secs(100))
        .connect(&url)
        .await
        .expect("can't get MySql Pool");
    pool
}
