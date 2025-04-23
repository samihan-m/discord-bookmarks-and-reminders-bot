use poise::serenity_prelude::{self, CreateEmbed};

#[derive(Debug)]
pub struct Reminder {
    user_id: u64,
    message: serenity_prelude::Message,
    remind_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct PersistedReminder {
    /// Sqlite integers are signed (otherwise I would make this a [`u64`])
    pk: i64,
    user_id: u64,
    message: serenity_prelude::Message,
    remind_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub enum ParseReminderError {
    #[expect(dead_code)]
    UserId(std::num::ParseIntError),
    #[expect(dead_code)]
    Message(serde_json::Error),
    #[expect(dead_code)]
    RemindAt(chrono::ParseError),
}

impl Reminder {
    pub fn new(
        user_id: u64,
        message: serenity_prelude::Message,
        remind_at: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            user_id,
            message,
            remind_at,
        }
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn message(&self) -> &serenity_prelude::Message {
        &self.message
    }

    pub fn remind_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.remind_at
    }
}

impl PersistedReminder {
    pub fn from_reminder(reminder: Reminder, pk: i64) -> Self {
        Self {
            pk,
            user_id: reminder.user_id,
            message: reminder.message,
            remind_at: reminder.remind_at,
        }
    }

    /// None of the [`String`]s need to be owned, but because this function is primarily
    /// a convenience function to be used when converting from a [`rusqlite::Row`]
    /// and it's infinitely less verbose to get an owned [`String`] than a [`&str`] from [`rusqlite::Row::get()`],
    /// (i.e., `&str` does not implement [`rusqlite::types::FromSql`])
    /// I just take owned Strings.
    /// This theoretically can be changed later to accomodate string slices if there is a desire for more flexibility
    /// in this function, but I think I'd rather just create a separate function.
    pub fn from_row(
        pk: i64,
        user_id: String,   // Sqlite integers are signed
        message: String,   // ideally, a json string
        remind_at: String, // ideally, a iso 8601 string
    ) -> Result<Self, ParseReminderError> {
        let user_id = user_id.parse::<u64>().map_err(ParseReminderError::UserId)?;

        let message = serde_json::from_str::<serenity_prelude::Message>(&message)
            .map_err(ParseReminderError::Message)?;

        let remind_at = chrono::DateTime::parse_from_rfc3339(&remind_at)
            .map_err(ParseReminderError::RemindAt)?
            .with_timezone(&chrono::Utc);

        Ok(Self {
            pk,
            user_id,
            message,
            remind_at,
        })
    }

    pub async fn to_embed(&self, http: &serenity_prelude::Http) -> CreateEmbed {
        let channel_name = self
            .message
            .channel_id
            .name(http)
            // This will error if we don't have permission to get DM channel information (which we currently do not)
            .await
            .map(|name| format!("#{}", name))
            .unwrap_or("the past!".to_string());

        self.create_embed(&channel_name)
    }

    fn create_embed(&self, channel_name: &str) -> CreateEmbed {
        let title = format!("Reminder from {}", channel_name);
        const MAX_TITLE_LENGTH: usize = 256;
        let trimmed_title = &title[..title.len().min(MAX_TITLE_LENGTH)];

        let description = format!("# {} \n # {}", self.message.content, self.message.link());
        const MAX_DESCRIPTION_LENGTH: usize = 4096;
        let trimmed_description = &description[..description.len().min(MAX_DESCRIPTION_LENGTH)];

        CreateEmbed::default()
            .title(trimmed_title)
            .description(trimmed_description)
            .timestamp(self.message.timestamp)
            .colour(serenity_prelude::Colour::TEAL)
    }

    pub fn pk(&self) -> i64 {
        self.pk
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn message(&self) -> &serenity_prelude::Message {
        &self.message
    }

    pub fn remind_at(&self) -> &chrono::DateTime<chrono::Utc> {
        &self.remind_at
    }
}

impl Ord for PersistedReminder {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.remind_at.cmp(&other.remind_at)
    }
}

impl PartialOrd for PersistedReminder {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PersistedReminder {
    fn eq(&self, other: &Self) -> bool {
        self.pk == other.pk
            && self.user_id == other.user_id
            && self.message.id == other.message.id
            && self.remind_at == other.remind_at
    }
}

impl Eq for PersistedReminder {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reminder_from_row() {
        let pk = 1;
        let user_id = "123456789".to_string();
        let message = serde_json::to_string(&serenity_prelude::Message::default()).unwrap();
        let some_time = chrono::Utc::now();
        let remind_at = some_time.to_rfc3339();
        let reminder = PersistedReminder::from_row(pk, user_id, message, remind_at).unwrap();
        assert_eq!(reminder.pk, pk);
        assert_eq!(reminder.user_id, 123456789);
        assert_eq!(reminder.message.id, serenity_prelude::Message::default().id);
        assert_eq!(reminder.remind_at, some_time);
    }

    #[test]
    fn test_create_embed() {
        let pk = 1;
        let user_id = 123456789;
        let message = serenity_prelude::Message::default();
        let remind_at = chrono::Utc::now();
        let reminder = PersistedReminder {
            pk,
            user_id,
            message: message.clone(),
            remind_at,
        };

        let channel_name = "#test_channel";

        let embed = reminder.create_embed(channel_name);

        let expected_embed = CreateEmbed::default()
            .title("Reminder from #test_channel")
            .description(&format!("# {} \n # {}", message.content, message.link()))
            .timestamp(message.timestamp)
            .colour(serenity_prelude::Colour::TEAL);
        assert_eq!(embed, expected_embed);
    }
}
