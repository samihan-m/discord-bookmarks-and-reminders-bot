pub mod bookmark;
pub mod delete_message_button;
pub mod interaction_custom_id;
pub mod relative_timestamp_string;
pub mod reminder;

pub const DELETE_MESSAGE_EMOJI: &str = "🗑️";

const MAX_EMBED_TITLE_LENGTH: usize = 256;
fn trim_embed_title(title: &str) -> &str {
    &title[..title.len().min(MAX_EMBED_TITLE_LENGTH)]
}

const MAX_EMBED_DESCRIPTION_LENGTH: usize = 4096;
fn trim_embed_description(description: &str) -> &str {
    &description[..description.len().min(MAX_EMBED_DESCRIPTION_LENGTH)]
}

const MAX_EMBED_FIELD_NAME_LENGTH: usize = 256;
fn trim_embed_field_name(field_name: &str) -> &str {
    &field_name[..field_name.len().min(MAX_EMBED_FIELD_NAME_LENGTH)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn test_trim_embed_title(title: String) -> bool {
        let trimmed_title = trim_embed_title(&title);
        trimmed_title.len() <= MAX_EMBED_TITLE_LENGTH
    }

    #[quickcheck]
    fn test_trim_embed_description(description: String) -> bool {
        let trimmed_description = trim_embed_description(&description);
        trimmed_description.len() <= MAX_EMBED_DESCRIPTION_LENGTH
    }

    #[quickcheck]
    fn test_trim_embed_field_name(field_name: String) -> bool {
        let trimmed_field_name = trim_embed_field_name(&field_name);
        trimmed_field_name.len() <= MAX_EMBED_FIELD_NAME_LENGTH
    }
}
