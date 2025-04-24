use poise::serenity_prelude as serenity;
use rusqlite::Row;
use uuid::Uuid;

#[derive(Debug)]
pub struct BookmarkedMessage {
    bookmark_id: Uuid,
    user_id: u64,
    message: serenity::Message,
}

#[derive(Debug)]
pub struct PersistedBookmarkedMessage {
    /// Sqlite integers are signed (otherwise I would make this a [`u64`])
    pk: i64,
    /// This is a UUIDv7 so we can sort by creation time without a timestamp field
    bookmark_id: Uuid,
    user_id: u64,
    message: serenity::Message,
}

#[derive(Debug)]
pub enum ParseBookmarkedMessageError {
    #[expect(dead_code)]
    BookmarkId(uuid::Error),
    #[expect(dead_code)]
    UserId(std::num::ParseIntError),
    #[expect(dead_code)]
    Message(serde_json::Error),
}

impl BookmarkedMessage {
    pub fn new(bookmark_id: Uuid, user_id: u64, message: serenity::Message) -> Self {
        Self {
            bookmark_id,
            user_id,
            message,
        }
    }

    pub fn bookmark_id(&self) -> Uuid {
        self.bookmark_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn message(&self) -> &serenity::Message {
        &self.message
    }
}

impl PersistedBookmarkedMessage {
    pub fn from_bookmarked_message(bookmark: BookmarkedMessage, pk: i64) -> Self {
        Self {
            pk,
            bookmark_id: bookmark.bookmark_id,
            user_id: bookmark.user_id,
            message: bookmark.message,
        }
    }

    pub fn from_row(
        pk: i64,
        bookmark_id: String, // ideally, a uuid string
        user_id: String,     // Sqlite integers are signed
        message: String,     // ideally, a json string
    ) -> Result<Self, ParseBookmarkedMessageError> {
        let bookmark_id =
            Uuid::parse_str(&bookmark_id).map_err(ParseBookmarkedMessageError::BookmarkId)?;

        let user_id = user_id
            .parse::<u64>()
            .map_err(ParseBookmarkedMessageError::UserId)?;

        let message = serde_json::from_str::<serenity::Message>(&message)
            .map_err(ParseBookmarkedMessageError::Message)?;

        Ok(Self {
            pk,
            bookmark_id,
            user_id,
            message,
        })
    }

    #[expect(dead_code)]
    pub fn pk(&self) -> i64 {
        self.pk
    }

    pub fn bookmark_id(&self) -> Uuid {
        self.bookmark_id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn message(&self) -> &serenity::Message {
        &self.message
    }
}

#[derive(Debug)]
pub enum PersistedBookmarkedMessageFromRowError {
    RusqliteError(rusqlite::Error),
    ParseBookmarkedMessageError(ParseBookmarkedMessageError),
}

impl From<rusqlite::Error> for PersistedBookmarkedMessageFromRowError {
    fn from(err: rusqlite::Error) -> Self {
        Self::RusqliteError(err)
    }
}

impl From<ParseBookmarkedMessageError> for PersistedBookmarkedMessageFromRowError {
    fn from(err: ParseBookmarkedMessageError) -> Self {
        Self::ParseBookmarkedMessageError(err)
    }
}

impl TryFrom<&Row<'_>> for PersistedBookmarkedMessage {
    type Error = PersistedBookmarkedMessageFromRowError;

    fn try_from(row: &Row) -> Result<Self, Self::Error> {
        let pk: i64 = row.get(0)?;
        let bookmark_id: String = row.get(1)?;
        let user_id: String = row.get(2)?;
        let message: String = row.get(3)?;

        Ok(Self::from_row(pk, bookmark_id, user_id, message)?)
    }
}
