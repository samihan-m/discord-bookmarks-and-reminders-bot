use poise::serenity_prelude as serenity;
use strum::IntoEnumIterator;

use crate::components::interaction_custom_id::InteractionCustomId;

use super::menu_value::ReminderSelectMenuValue;

pub fn get_reminder_select_menu(custom_id: InteractionCustomId) -> serenity::CreateSelectMenu {
    let options = ReminderSelectMenuValue::iter()
        .map(get_select_menu_value)
        .collect::<Vec<_>>();

    serenity::CreateSelectMenu::new(
        custom_id,
        serenity::CreateSelectMenuKind::String { options },
    )
    .min_values(1)
    .max_values(1)
    .placeholder("When would you like to be reminded?")
}

fn get_select_menu_value(value: ReminderSelectMenuValue) -> serenity::CreateSelectMenuOption {
    serenity::CreateSelectMenuOption::new(format!("In {}", value), value.to_string())
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    #[test]
    fn test_get_reminder_select_menu() {
        let custom_id =
            InteractionCustomId::SetReminder(Uuid::new_v7(uuid::Timestamp::now(uuid::NoContext)));
        let select_menu = get_reminder_select_menu(custom_id.clone());

        let expected_select_menu = serenity::CreateSelectMenu::new(
            custom_id,
            serenity::CreateSelectMenuKind::String {
                options: ReminderSelectMenuValue::iter()
                    .map(get_select_menu_value)
                    .collect(),
            },
        )
        .min_values(1)
        .max_values(1)
        .placeholder("When would you like to be reminded?");

        assert_eq!(select_menu, expected_select_menu);
    }

    #[test]
    fn test_get_select_menu_value() {
        let value = ReminderSelectMenuValue::TenSeconds;
        let select_menu_value = get_select_menu_value(value.clone());

        let expected_select_menu_value =
            serenity::CreateSelectMenuOption::new(format!("In {}", value), value.to_string());

        assert_eq!(select_menu_value, expected_select_menu_value);
    }
}
