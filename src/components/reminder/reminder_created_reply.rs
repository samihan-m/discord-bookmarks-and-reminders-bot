use poise::CreateReply;

use crate::components::relative_timestamp_string::get_discord_relative_timestamp_string;

pub fn get_reminder_created_reply(remind_at: &chrono::DateTime<chrono::Utc>) -> CreateReply {
    CreateReply::default()
        .content(format!(
            "Reminder set for {}",
            get_discord_relative_timestamp_string(remind_at)
        ))
        .reply(true)
        .ephemeral(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_reminder_created_reply() {
        let remind_at = chrono::Utc::now();
        let result = get_reminder_created_reply(&remind_at);
        assert_eq!(result.ephemeral, Some(true));
        assert_eq!(result.reply, true);
        assert_eq!(
            result.content.unwrap(),
            format!("Reminder set for <t:{}:R>", remind_at.timestamp())
        );
    }
}
