use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySql, Pool};

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Book {
    pub id: Option<i64>,             // bigint
    pub name: String,                // varchar(100) NOT NULL
    pub author: String,              // varchar(100) NOT NULL
    pub cover_url: Option<String>,   // varchar(255) DEFAULT NULL
    pub path_url: Option<String>,    // varchar(255) DEFAULT NULL
    pub description: Option<String>, // text DEFAULT NULL
    pub category_id: Option<i64>,    // bigint DEFAULT NULL
    pub word_count: i32,             // int DEFAULT 0
    pub chapter_count: i32,          // int DEFAULT 0
    pub status: i8,                  // tinyint DEFAULT 0
    pub view_count: i64,             // bigint DEFAULT 0
    pub price: i32,                  // int NOT NULL DEFAULT 0
    pub is_deleted: i32,             // int NOT NULL DEFAULT 0
    pub created_at: String,          // datetime
    pub updated_at: String,          // datetime
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BookCategory {
    pub id: i64,                     // bigint NOT NULL
    pub name: String,                // varchar(50) NOT NULL
    pub pid: i64,                    // bigint NOT NULL DEFAULT 0
    pub code: Option<String>,        // varchar(50) DEFAULT NULL
    pub sort: i32,                   // int DEFAULT 0
    pub icon: Option<String>,        // varchar(255) DEFAULT NULL
    pub description: Option<String>, // varchar(500) DEFAULT NULL
    pub book_count: i32,             // int DEFAULT 0
    pub status: i8,                  // tinyint DEFAULT 0
    pub is_hot: i8,                  // tinyint DEFAULT 0
    pub created_at: String,          // datetime
    pub updated_at: String,          // datetime
    pub is_deleted: i8,              // tinyint DEFAULT 0
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct BookChapter {
    pub id: Option<i64>,                   // bigint NOT NULL
    pub book_id: i64,              // bigint NOT NULL
    pub title: String,             // varchar(200) NOT NULL
    pub chapter_index: i32,        // int NOT NULL
    pub word_count: i32,           // int DEFAULT 0
    pub file_path: Option<String>, // varchar(255) DEFAULT NULL
    pub created_at: String,        // datetime -> String
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Bookshelf {
    pub id: i64,                      // bigint NOT NULL
    pub user_id: i64,                 // bigint NOT NULL
    pub book_id: i64,                 // bigint NOT NULL
    pub is_purchased: i32,            // int NOT NULL DEFAULT 0
    pub last_chapter_id: Option<i32>, // int DEFAULT NULL
    pub is_deleted: i32,              // int NOT NULL DEFAULT 0
    pub created_at: String,           // datetime -> String
}

impl Book {
    pub async fn create_book(pool: &Pool<MySql>, book: &Book) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        INSERT INTO t_book
        (name, author, cover_url, path_url, description, category_id,
         word_count, chapter_count, status, view_count, price, is_deleted)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        )
        .bind(&book.name)
        .bind(&book.author)
        .bind(&book.cover_url)
        .bind(&book.path_url)
        .bind(&book.description)
        .bind(&book.category_id)
        .bind(book.word_count)
        .bind(book.chapter_count)
        .bind(book.status)
        .bind(book.view_count)
        .bind(book.price)
        .bind(book.is_deleted)
        .execute(pool)
        .await?;
        Ok(result.last_insert_id())
    }

    pub async fn get_book_by_id(pool: &Pool<MySql>, id: i64) -> Result<Option<Book>, sqlx::Error> {
        sqlx::query_as::<_, Book>("SELECT * FROM t_book WHERE id = ? AND is_deleted = 0")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    pub async fn update_book(pool: &Pool<MySql>, book: &Book) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        UPDATE t_book SET
            name = ?,
            author = ?,
            cover_url = ?,
            description = ?,
            category_id = ?,
            word_count = ?,
            chapter_count = ?,
            status = ?,
            price = ?
        WHERE id = ?
        "#,
        )
        .bind(&book.name)
        .bind(&book.author)
        .bind(&book.cover_url)
        .bind(&book.description)
        .bind(&book.category_id)
        .bind(book.word_count)
        .bind(book.chapter_count)
        .bind(book.status)
        .bind(book.price)
        .bind(book.id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn delete_book(pool: &Pool<MySql>, id: i64) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("UPDATE t_book SET is_deleted = 1 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}

impl BookChapter {
    pub async fn create_chapter(
        pool: &Pool<MySql>,
        chapter: &BookChapter,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        INSERT INTO t_book_chapter
        (book_id, title, chapter_index, word_count, file_path)
        VALUES (?, ?, ?, ?, ?)
        "#,
        )
        .bind(chapter.book_id)
        .bind(&chapter.title)
        .bind(chapter.chapter_index)
        .bind(chapter.word_count)
        .bind(&chapter.file_path)
        .execute(pool)
        .await?;

        Ok(result.last_insert_id())
    }

    pub async fn get_chapters_by_book(
        pool: &Pool<MySql>,
        book_id: i64,
    ) -> Result<Vec<BookChapter>, sqlx::Error> {
        sqlx::query_as::<_, BookChapter>(
            "SELECT * FROM t_book_chapter WHERE book_id = ? ORDER BY chapter_index ASC",
        )
        .bind(book_id)
        .fetch_all(pool)
        .await
    }
}

impl Bookshelf {
    pub async fn add_to_bookshelf(
        pool: &Pool<MySql>,
        user_id: i64,
        book_id: i64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        INSERT INTO t_bookshelf (user_id, book_id)
        VALUES (?, ?)
        ON DUPLICATE KEY UPDATE is_deleted = 0
        "#,
        )
        .bind(user_id)
        .bind(book_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn update_read_progress(
        pool: &Pool<MySql>,
        user_id: i64,
        book_id: i64,
        chapter_id: i32,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
        UPDATE t_bookshelf
        SET last_chapter_id = ?
        WHERE user_id = ? AND book_id = ?
        "#,
        )
        .bind(chapter_id)
        .bind(user_id)
        .bind(book_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    pub async fn get_user_bookshelf(
        pool: &Pool<MySql>,
        user_id: i64,
    ) -> Result<Vec<Book>, sqlx::Error> {
        sqlx::query_as::<_, Book>(
            r#"
        SELECT b.*
        FROM t_bookshelf s
        JOIN t_book b ON s.book_id = b.id
        WHERE s.user_id = ?
          AND s.is_deleted = 0
          AND b.is_deleted = 0
        ORDER BY s.created_at DESC
        "#,
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
    }
}
