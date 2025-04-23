use tokio::sync::Mutex;
use tokio_rusqlite::Connection;

use crate::models::reminder::{PersistedReminder, Reminder};

pub async fn create_reminders_table_if_nonexistent(
    db_connection: &Mutex<Connection>,
) -> Result<(), tokio_rusqlite::Error> {
    db_connection
        .lock()
        .await
        .call(|conn| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS reminders (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        user_id TEXT NOT NULL,
                        message TEXT NOT NULL,
                        remind_at TEXT NOT NULL
                    ) STRICT",
                [],
            )?;
            Ok(())
        })
        .await
}

pub async fn get_all_reminders(
    db_connection: &Mutex<Connection>,
) -> Result<Vec<PersistedReminder>, tokio_rusqlite::Error> {
    db_connection
        .lock()
        .await
        .call(|conn| {
            let mut stmt = conn.prepare("SELECT * FROM reminders")?;
            let reminders_from_database = stmt
                .query(())?
                .mapped(|row| {
                    Ok(PersistedReminder::from_row(
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                    )
                    .expect("Failed to parse reminder from row"))
                })
                .filter_map(Result::ok)
                .collect::<Vec<_>>();

            Ok(reminders_from_database)
        })
        .await
}

pub async fn delete_reminder_by_id(
    db_connection: &Mutex<Connection>,
    reminder_id: i64,
) -> Result<(), tokio_rusqlite::Error> {
    let rows_changed = db_connection
        .lock()
        .await
        .call(move |conn| Ok(conn.execute("DELETE FROM reminders WHERE id = ?", [reminder_id])?))
        .await?;

    assert!(
        rows_changed == 1,
        "Expected exactly one row to be deleted but instead {} were deleted.",
        rows_changed
    );

    Ok(())
}

pub async fn get_reminders_for_user(
    db_connection: &Mutex<Connection>,
    user_id: u64,
    max_quantity_to_retrieve: u64,
) -> Result<Vec<PersistedReminder>, tokio_rusqlite::Error> {
    db_connection
        .lock()
        .await
        .call(move |conn| {
            let reminders = conn
                .prepare(
                    "SELECT * FROM reminders WHERE user_id = ?1 ORDER BY remind_at DESC LIMIT ?2",
                )?
                .query_map([user_id, max_quantity_to_retrieve], |row| {
                    Ok(PersistedReminder::from_row(
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                    )
                    .expect("Failed to parse reminder from row"))
                })?
                .collect::<Result<Vec<_>, _>>()?;

            Ok(reminders)
        })
        .await
}

pub async fn insert_reminder(
    db_connection: &Mutex<Connection>,
    reminder: Reminder,
) -> Result<PersistedReminder, tokio_rusqlite::Error> {
    let user_id = reminder.user_id();
    let stringified_message =
        serde_json::to_string(&reminder.message()).expect("Failed to serialize message");
    let remind_at = *reminder.remind_at();

    let pk = db_connection
        .lock()
        .await
        .call(move |conn| {
            conn.execute(
                "INSERT INTO reminders (user_id, message, remind_at) VALUES (?1, ?2, ?3)",
                tokio_rusqlite::params![user_id, stringified_message, remind_at.to_rfc3339(),],
            )?;

            Ok(conn.last_insert_rowid())
        })
        .await?;

    Ok(PersistedReminder::from_reminder(reminder, pk))
}
