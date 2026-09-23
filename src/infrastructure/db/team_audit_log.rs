use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use uuid::Uuid;

use crate::domain::team_audit_log::{AuditAction, AuditActorType, TeamAuditLogEntry};
use crate::error::AppError;

/// Snapshot der auditierbaren Team-Felder für Vorher/Nachher-Vergleiche.
/// Kein eigenes DB-Entity — reine Hilfsstruktur zum Bauen von `changes`,
/// gehört daher nicht ins Repository selbst.
#[derive(Debug, Clone, PartialEq, sqlx::FromRow, Serialize)]
pub(super) struct TeamAuditSnapshot {
    pub name: String,
    pub mail: Option<String>,
    pub phone: Option<String>,
    pub members: Option<i32>,
    pub diets: Option<String>,
    pub address_text: String,
    pub latitude: f64,
    pub longitude: f64,
}

pub(super) fn diff_json(before: &TeamAuditSnapshot, after: &TeamAuditSnapshot) -> serde_json::Value {
    json!({ "before": before, "after": after })
}

pub struct TeamAuditLogRepository;

#[derive(Debug, Clone, sqlx::FromRow)]
struct TeamAuditLogEntity {
    id: Uuid,
    actor_type: AuditActorType,
    actor_label: Option<String>,
    action: AuditAction,
    changes: serde_json::Value,
    created_at: DateTime<Utc>,
}

impl TeamAuditLogEntity {
    fn to_domain(&self) -> TeamAuditLogEntry {
        TeamAuditLogEntry {
            id: self.id,
            actor_type: self.actor_type,
            actor_label: self.actor_label.clone(),
            action: self.action,
            changes: self.changes.clone(),
            created_at: self.created_at,
        }
    }
}

impl TeamAuditLogRepository {
    #[tracing::instrument(skip(self, executor, changes))]
    pub async fn insert<'e, E>(
        &self,
        executor: E,
        team_id: &Uuid,
        actor_type: AuditActorType,
        actor_label: Option<&str>,
        action: AuditAction,
        changes: &serde_json::Value,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        sqlx::query(
            "INSERT INTO team_audit_log
                (id, team_id, actor_type, actor_label, action, changes, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(Uuid::new_v4())
        .bind(team_id)
        .bind(actor_type)
        .bind(actor_label)
        .bind(action)
        .bind(changes)
        .bind(time)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// Liefert eine paginierte Seite der Log-Einträge plus Gesamtanzahl für
    /// `team_id`, in einer einzigen Query (Window-Funktion) — ein generisches
    /// `E` ist nicht zwangsläufig `Copy` (z. B. `&mut Transaction`), zwei
    /// sequenzielle Queries auf demselben `executor`-Wert wären daher nicht
    /// ohne Weiteres möglich. Prüft KEIN Besitzverhältnis mehr — der
    /// Aufrufer hat das Team i.d.R. bereits über `TeamRepository::select`
    /// geladen und damit implizit geprüft.
    #[tracing::instrument(skip(self, executor))]
    pub async fn select_for_team<'e, E>(
        &self,
        executor: E,
        team_id_filter: &Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<(Vec<TeamAuditLogEntry>, i64), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        #[derive(sqlx::FromRow)]
        struct PageRow {
            id: Uuid,
            actor_type: AuditActorType,
            actor_label: Option<String>,
            action: AuditAction,
            changes: serde_json::Value,
            created_at: DateTime<Utc>,
            total_count: i64,
        }

        let rows: Vec<PageRow> = sqlx::query_as(
            "SELECT id, actor_type, actor_label, action, changes, created_at,
                    COUNT(*) OVER() AS total_count
             FROM team_audit_log
             WHERE team_id = $1
             ORDER BY created_at DESC
             LIMIT $2 OFFSET $3",
        )
        .bind(team_id_filter)
        .bind(limit)
        .bind(offset)
        .fetch_all(executor)
        .await
        .map_err(AppError::DatabaseError)?;

        let total = rows.first().map(|r| r.total_count).unwrap_or(0);
        let entries = rows
            .iter()
            .map(|r| TeamAuditLogEntity {
                id: r.id,
                actor_type: r.actor_type,
                actor_label: r.actor_label.clone(),
                action: r.action,
                changes: r.changes.clone(),
                created_at: r.created_at,
            })
            .map(|e| e.to_domain())
            .collect();

        Ok((entries, total))
    }
}