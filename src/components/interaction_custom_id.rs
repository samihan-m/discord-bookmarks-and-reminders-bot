use uuid::Uuid;

pub const DELETE_MESSAGE_INTERACTION_CUSTOM_ID: &str = "delete_message";
pub const SET_REMINDER_INTERACTION_CUSTOM_ID: &str = "set_reminder";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InteractionCustomId {
    DeleteMessage,
    SetReminder(Uuid),
}

impl From<InteractionCustomId> for String {
    fn from(value: InteractionCustomId) -> Self {
        match value {
            InteractionCustomId::DeleteMessage => DELETE_MESSAGE_INTERACTION_CUSTOM_ID.to_string(),
            InteractionCustomId::SetReminder(uuid) => {
                format!("{}:{}", SET_REMINDER_INTERACTION_CUSTOM_ID, uuid)
            }
        }
    }
}

impl TryFrom<&str> for InteractionCustomId {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let parts: Vec<&str> = value.split(':').collect();
        match parts.as_slice() {
            [DELETE_MESSAGE_INTERACTION_CUSTOM_ID] => Ok(Self::DeleteMessage),
            [SET_REMINDER_INTERACTION_CUSTOM_ID, maybe_uuid] => {
                let uuid = Uuid::parse_str(maybe_uuid).map_err(|_| {
                    format!(
                        "Received invalid UUID for {}: {}",
                        SET_REMINDER_INTERACTION_CUSTOM_ID, maybe_uuid
                    )
                })?;
                Ok(Self::SetReminder(uuid))
            }
            _ => Err(format!("Received invalid custom ID: {}", value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This function exists to trigger a compiler error when we add a new
    /// variant of [`InteractionCustomId`]. When fixing the error in this function,
    /// make sure to update the rest of the tests in this module to cover the new variant as well.
    #[expect(dead_code)]
    fn unhandled_variants_alarm(some_input: InteractionCustomId) {
        match some_input {
            InteractionCustomId::DeleteMessage => (),
            InteractionCustomId::SetReminder(_) => (),
        }
    }

    #[test]
    fn test_interaction_custom_id_from() {
        let delete_message_id = InteractionCustomId::DeleteMessage;
        let uuid = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let set_reminder_id = InteractionCustomId::SetReminder(uuid);

        assert_eq!(
            String::from(delete_message_id),
            DELETE_MESSAGE_INTERACTION_CUSTOM_ID
        );
        assert_eq!(
            String::from(set_reminder_id),
            format!("{}:{}", SET_REMINDER_INTERACTION_CUSTOM_ID, uuid)
        );
    }

    #[test]
    fn test_interaction_custom_id_try_from() {
        let delete_message_id = DELETE_MESSAGE_INTERACTION_CUSTOM_ID;
        let uuid = Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext));
        let set_reminder_id = format!("{}:{}", SET_REMINDER_INTERACTION_CUSTOM_ID, uuid);

        assert_eq!(
            InteractionCustomId::try_from(delete_message_id).unwrap(),
            InteractionCustomId::DeleteMessage
        );
        assert_eq!(
            InteractionCustomId::try_from(set_reminder_id.as_str()).unwrap(),
            InteractionCustomId::SetReminder(uuid)
        );
        assert!(InteractionCustomId::try_from("invalid_id").is_err());
    }
}
