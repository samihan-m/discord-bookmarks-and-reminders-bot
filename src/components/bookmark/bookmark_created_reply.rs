use poise::CreateReply;

pub fn get_bookmark_created_reply() -> CreateReply {
    CreateReply::default()
        .content("Bookmark created! Check DMs.")
        .reply(true)
        .ephemeral(true)
}

pub fn get_bookmark_already_exists_reply() -> CreateReply {
    CreateReply::default()
        .content("Bookmark already exists! Check DMs.")
        .reply(true)
        .ephemeral(true)
}

pub fn get_failed_to_create_bookmark_reply() -> CreateReply {
    CreateReply::default()
        .content("Failed to create bookmark.")
        .reply(true)
        .ephemeral(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_bookmark_created_reply() {
        let result = get_bookmark_created_reply();
        assert_eq!(result.ephemeral, Some(true));
        assert_eq!(result.reply, true);
        assert_eq!(
            result.content.unwrap(),
            "Bookmark created! Check DMs.".to_string()
        );
    }

    #[test]
    fn test_get_bookmark_already_exists_reply() {
        let result = get_bookmark_already_exists_reply();
        assert_eq!(result.ephemeral, Some(true));
        assert_eq!(result.reply, true);
        assert_eq!(
            result.content.unwrap(),
            "Bookmark already exists! Check DMs.".to_string()
        );
    }

    #[test]
    fn test_get_failed_to_create_bookmark_reply() {
        let result = get_failed_to_create_bookmark_reply();
        assert_eq!(result.ephemeral, Some(true));
        assert_eq!(result.reply, true);
        assert_eq!(
            result.content.unwrap(),
            "Failed to create bookmark.".to_string()
        );
    }
}
