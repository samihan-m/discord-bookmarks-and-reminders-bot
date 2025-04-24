use strum_macros::{Display, EnumIter, EnumString};

#[derive(Debug, Clone, PartialEq, Eq, Display, EnumIter, EnumString)]
pub enum ReminderSelectMenuValue {
    #[strum(serialize = "10 seconds")]
    TenSeconds,
    #[strum(serialize = "1 hour")]
    OneHour,
    #[strum(serialize = "24 hours")]
    TwentyFourHours,
}

impl From<ReminderSelectMenuValue> for chrono::Duration {
    fn from(value: ReminderSelectMenuValue) -> Self {
        match value {
            ReminderSelectMenuValue::TenSeconds => chrono::Duration::seconds(10),
            ReminderSelectMenuValue::OneHour => chrono::Duration::hours(1),
            ReminderSelectMenuValue::TwentyFourHours => chrono::Duration::hours(24),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn test_reminder_select_menu_value_display() {
        ReminderSelectMenuValue::iter().for_each(|value| {
            let displayed_value = value.to_string();
            let expected_value = match value {
                ReminderSelectMenuValue::TenSeconds => "10 seconds",
                ReminderSelectMenuValue::OneHour => "1 hour",
                ReminderSelectMenuValue::TwentyFourHours => "24 hours",
            };
            assert_eq!(displayed_value, expected_value);
        });
    }

    #[test]
    fn test_reminder_select_menu_value_parse() {
        ReminderSelectMenuValue::iter().for_each(|value| {
            let parsed_value = ReminderSelectMenuValue::from_str(&value.to_string()).unwrap();
            let expected_value = match value {
                ReminderSelectMenuValue::TenSeconds => ReminderSelectMenuValue::TenSeconds,
                ReminderSelectMenuValue::OneHour => ReminderSelectMenuValue::OneHour,
                ReminderSelectMenuValue::TwentyFourHours => {
                    ReminderSelectMenuValue::TwentyFourHours
                }
            };
            assert_eq!(parsed_value, expected_value);
        });
    }

    #[test]
    fn test_reminder_select_menu_value_from() {
        ReminderSelectMenuValue::iter().for_each(|value| {
            let expected_duration = match value {
                ReminderSelectMenuValue::TenSeconds => chrono::Duration::seconds(10),
                ReminderSelectMenuValue::OneHour => chrono::Duration::hours(1),
                ReminderSelectMenuValue::TwentyFourHours => chrono::Duration::hours(24),
            };
            let duration: chrono::Duration = value.into();
            assert_eq!(duration, expected_duration);
        });
    }
}
