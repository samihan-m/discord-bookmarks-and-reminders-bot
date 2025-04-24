use poise::CreateReply;

pub fn get_no_bookmarks_found_reply() -> CreateReply {
    CreateReply::default()
        .content("No bookmarks found in the database.")
        .ephemeral(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_no_bookmarks_found_reply() {
        let reply = get_no_bookmarks_found_reply();
        assert_eq!(
            reply.content,
            Some("No bookmarks found in the database.".to_string())
        );
        assert_eq!(reply.ephemeral, Some(true));
    }
}
