use chrono::{DateTime, Utc};
use tokio_postgres::{GenericClient, Row};

use crate::models::AlarmKind;

pub(crate) struct AlarmWrite {
    pub id: String,
    pub device_id: String,
    pub kind: AlarmKind,
    pub series_id: String,
    pub generation: i64,
    pub severity: String,
    pub title: String,
    pub message: String,
    pub fire_at: DateTime<Utc>,
    pub hidden_buffer_applied: bool,
    pub requires_acknowledgement: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

pub(crate) async fn persist_alarm<C>(
    client: &C,
    alarm: &AlarmWrite,
) -> Result<Row, tokio_postgres::Error>
where
    C: GenericClient + Sync,
{
    let row = client
        .query_one(
            "INSERT INTO alarms (
                 id, device_id, kind, series_id, generation, severity, title, message,
                 fire_at, hidden_buffer_applied, requires_acknowledgement, expires_at, status
             ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, 'pending')
             ON CONFLICT (id) DO UPDATE SET
                 device_id = EXCLUDED.device_id, kind = EXCLUDED.kind,
                 series_id = EXCLUDED.series_id, generation = EXCLUDED.generation,
                 severity = EXCLUDED.severity, title = EXCLUDED.title,
                 message = EXCLUDED.message, fire_at = EXCLUDED.fire_at,
                 hidden_buffer_applied = EXCLUDED.hidden_buffer_applied,
                 requires_acknowledgement = EXCLUDED.requires_acknowledgement,
                 expires_at = EXCLUDED.expires_at, status = 'pending', delivery_attempts = 0,
                 last_delivered_at = NULL, delivery_token = NULL,
                 delivery_lease_expires_at = NULL, scheduled_local_id = NULL,
                 scheduled_at = NULL, cancellation_confirmed_at = NULL, updated_at = now()
             RETURNING id, kind, series_id, generation, delivery_token, severity, title,
                 message, fire_at, hidden_buffer_applied, requires_acknowledgement, expires_at",
            &[
                &alarm.id,
                &alarm.device_id,
                &alarm.kind.as_str(),
                &alarm.series_id,
                &alarm.generation,
                &alarm.severity,
                &alarm.title,
                &alarm.message,
                &alarm.fire_at,
                &alarm.hidden_buffer_applied,
                &alarm.requires_acknowledgement,
                &alarm.expires_at,
            ],
        )
        .await?;
    let outbox_id = format!("wake:{}:{}", alarm.series_id, alarm.device_id);
    client
        .execute(
            "INSERT INTO alarm_wake_outbox (id, device_id, alarm_id)
             VALUES ($1, $2, $3)
             ON CONFLICT (id) DO UPDATE SET alarm_id = EXCLUDED.alarm_id,
                 status = 'pending', lease_token = NULL, lease_expires_at = NULL,
                 updated_at = now()",
            &[&outbox_id, &alarm.device_id, &alarm.id],
        )
        .await?;
    Ok(row)
}
