use rusqlite::{OptionalExtension, Row};
use tokio::sync::Mutex;
use tokio_rusqlite::Connection;
use uuid::Uuid;

use crate::models::bookmark::{
    BookmarkedMessage, PersistedBookmarkedMessage, PersistedBookmarkedMessageFromRowError,
};

pub async fn create_bookmarks_table_if_nonexistent(
    db_connection: &Mutex<Connection>,
) -> Result<(), tokio_rusqlite::Error> {
    db_connection
        .lock()
        .await
        .call(|conn| {
            conn.execute_batch(
                "
                BEGIN;
                CREATE TABLE IF NOT EXISTS bookmarks (
                    pk INTEGER PRIMARY KEY AUTOINCREMENT,
                    bookmark_id TEXT NOT NULL,
                    user_id TEXT NOT NULL,
                    message TEXT NOT NULL
                ) STRICT;
                 
                CREATE UNIQUE INDEX IF NOT EXISTS one_bookmark_per_message_per_user ON bookmarks(json_extract(message, '$.id'), user_id);
                COMMIT;
                 ",
            )?;
            Ok(())
        })
        .await
}

#[derive(Debug)]
pub enum InsertBookmarkError {
    BookmarkAlreadyExists(PersistedBookmarkedMessage),
    #[expect(dead_code)]
    TokioRusqliteError(tokio_rusqlite::Error),
}

impl From<tokio_rusqlite::Error> for InsertBookmarkError {
    fn from(err: tokio_rusqlite::Error) -> Self {
        Self::TokioRusqliteError(err)
    }
}

pub async fn insert_bookmark(
    db_connection: &Mutex<Connection>,
    bookmark: BookmarkedMessage,
) -> Result<PersistedBookmarkedMessage, InsertBookmarkError> {
    let bookmark_id = bookmark.bookmark_id().to_string();
    let user_id = bookmark.user_id();
    let stringified_message =
        serde_json::to_string(&bookmark.message()).expect("Failed to serialize message");

    let message_id = bookmark.message().id.to_string();
    let existing_bookmark = db_connection
        .lock()
        .await
        .call(move |conn| {
            let bookmark = conn
                .query_row(
                    "SELECT * FROM bookmarks WHERE user_id = ?1 AND json_extract(message, '$.id') = ?2",
                    tokio_rusqlite::params![user_id, message_id],
                    bookmark_from_row,
                )
                .optional()?;

            Ok(bookmark)
        })
        .await?;

    if let Some(existing_bookmark) = existing_bookmark {
        return Err(InsertBookmarkError::BookmarkAlreadyExists(
            existing_bookmark,
        ));
    }

    let pk = db_connection
        .lock()
        .await
        .call(move |conn| {
            conn.execute(
                "INSERT INTO bookmarks (bookmark_id, user_id, message) VALUES (?1, ?2, ?3)",
                tokio_rusqlite::params![bookmark_id, user_id, stringified_message],
            )?;

            Ok(conn.last_insert_rowid())
        })
        .await?;

    Ok(PersistedBookmarkedMessage::from_bookmarked_message(
        bookmark, pk,
    ))
}

pub async fn get_bookmark_by_id(
    db_connection: &Mutex<Connection>,
    bookmark_id: Uuid,
) -> Result<Option<PersistedBookmarkedMessage>, tokio_rusqlite::Error> {
    db_connection
        .lock()
        .await
        .call(move |conn| {
            let bookmark = conn
                .query_row(
                    "SELECT * FROM bookmarks WHERE bookmark_id = ?1",
                    [bookmark_id.to_string()],
                    bookmark_from_row,
                )
                .optional()?;

            Ok(bookmark)
        })
        .await
}

fn bookmark_from_row(row: &Row<'_>) -> Result<PersistedBookmarkedMessage, rusqlite::Error> {
    match PersistedBookmarkedMessage::try_from(row) {
        Ok(bookmark) => Ok(bookmark),
        Err(PersistedBookmarkedMessageFromRowError::RusqliteError(rusqlite_error)) => {
            Err(rusqlite_error)
        }
        Err(PersistedBookmarkedMessageFromRowError::ParseBookmarkedMessageError(err)) => {
            panic!("Failed to parse existing bookmark: {:?}", err)
        }
    }
}
