pub fn get_discord_relative_timestamp_string(remind_at: &chrono::DateTime<chrono::Utc>) -> String {
    format!("<t:{}:R>", remind_at.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_discord_relative_timestamp_string() {
        let remind_at = chrono::Utc::now();
        let result = get_discord_relative_timestamp_string(&remind_at);
        assert_eq!(result, format!("<t:{}:R>", remind_at.timestamp()));
    }
}
