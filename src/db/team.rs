use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{Postgres, Transaction};
use tracing::warn;
use uuid::Uuid;

use crate::db::address::create_address;
use crate::db::email_outbox::insert_email_outbox_tx;
use crate::db::models::{
    Address, AdminNotificationTarget, AuditAction, AuditActorType, EmailType, Team, TeamFields,
    TeamStatus,
};
use crate::db::plan_staleness;
use crate::db::team_audit_log::{diff_json, insert_audit_row, TeamAuditSnapshot};
use crate::error::AppError;

fn field_display_name(field: &TeamFields) -> &'static str {
    match field {
        TeamFields::Mail => "email address",
        TeamFields::Phone => "phone number",
        TeamFields::Members => "member count",
        TeamFields::Diets => "dietary requirements",
    }
}

/// Marks the plan as stale if it references the given team. Only writes an
/// audit entry if this call actually triggered the transition (prevents log
/// spam on repeated calls).
async fn mark_plan_stale_if_referenced(
    tx: &mut Transaction<'_, Postgres>,
    cook_and_run_id: &Uuid,
    team_id: &Uuid,
    reason: &str,
    time: &DateTime<Utc>,
) -> Result<(), AppError> {
    let Some(plan_data) = plan_staleness::plan_data_if_present(tx, cook_and_run_id).await? else {
        return Ok(());
    };
    if !plan_staleness::plan_references_team(&plan_data, team_id) {
        return Ok(());
    }
    if plan_staleness::mark_plan_stale(tx, cook_and_run_id, time).await? {
        let changes = json!({ "reason": reason });
        insert_audit_row(
            tx,
            team_id,
            AuditActorType::System,
            None,
            AuditAction::PlanInvalidated,
            &changes,
            time,
        )
        .await?;
    }
    Ok(())
}

impl super::Database {
    #[tracing::instrument(skip(self, data, address_data, email_to_enqueue))]
    pub async fn create_team(
        &self,
        data: &Team,
        address_data: &Address,
        actor_type: AuditActorType,
        actor_label: Option<&str>,
        email_to_enqueue: Option<(&str, EmailType, &serde_json::Value)>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        create_address(&mut *tx, address_data).await?;

        let result = sqlx::query(
            "INSERT INTO team
                (id, cook_and_run_id, created_by_user, name, created, edited,
                 address, mail, phone, members, diets,
                 status, canceled_at, cancel_reason, access_token,
                 email_verified_at, verification_resend_count, last_route_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                     $12, $13, $14, $15, $16, $17, $18)",
        )
        .bind(data.id)
        .bind(data.cook_and_run_id)
        .bind(&data.created_by_user)
        .bind(&data.name)
        .bind(data.created)
        .bind(data.edited)
        .bind(data.address)
        .bind(&data.mail)
        .bind(&data.phone)
        .bind(data.members)
        .bind(&data.diets)
        .bind(data.status)
        .bind(data.canceled_at)
        .bind(&data.cancel_reason)
        .bind(&data.access_token)
        .bind(data.email_verified_at)
        .bind(data.verification_resend_count)
        .bind(&data.last_route_hash)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(_) => {
                let changes = json!({
                    "after": {
                        "name": data.name,
                        "mail": data.mail,
                        "phone": data.phone,
                        "members": data.members,
                        "diets": data.diets,
                        "status": format!("{:?}", data.status),
                        "address_text": address_data.address_text,
                        "latitude": address_data.latitude,
                        "longitude": address_data.longitude,
                    }
                });
                insert_audit_row(
                    &mut tx,
                    &data.id,
                    actor_type,
                    actor_label,
                    AuditAction::Created,
                    &changes,
                    &data.created,
                )
                .await?;

                if plan_staleness::mark_plan_stale(&mut tx, &data.cook_and_run_id, &data.created)
                    .await?
                {
                    let stale_changes = json!({ "reason": "team_added" });
                    insert_audit_row(
                        &mut tx,
                        &data.id,
                        AuditActorType::System,
                        None,
                        AuditAction::PlanInvalidated,
                        &stale_changes,
                        &data.created,
                    )
                    .await?;
                }

                if let Some((recipient, email_type, context)) = email_to_enqueue {
                    insert_email_outbox_tx(
                        &mut tx,
                        Some(data.id),
                        recipient,
                        email_type,
                        context,
                        &data.created,
                    )
                    .await?;
                }

                tx.commit().await.map_err(AppError::DatabaseError)?;
                Ok(())
            }
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                tx.rollback().await.map_err(AppError::DatabaseError)?;
                Ok(())
            }
            Err(e) => {
                tx.rollback().await.map_err(AppError::DatabaseError)?;
                Err(AppError::DatabaseError(e))
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn count_teams(&self, cook_and_run_id_filter: &Uuid) -> Result<i64, AppError> {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM team t
             INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
             WHERE car.id = $1",
        )
        .bind(cook_and_run_id_filter)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::DatabaseError)
    }

    const TEAM_ADDRESS_COLUMNS: &'static str = "
        t.id, t.cook_and_run_id, t.created_by_user, t.name, t.created, t.edited,
        t.address, t.mail, t.phone, t.members, t.diets,
        t.status, t.canceled_at, t.cancel_reason, t.access_token,
        t.email_verified_at, t.verification_resend_count, t.last_route_hash,
        a.id AS a_id, a.address_text AS a_address_text,
        a.latitude AS a_latitude, a.longitude AS a_longitude";

    #[tracing::instrument(skip(self))]
    pub async fn select_all_team(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<(Team, Address)>, AppError> {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
             INNER JOIN address a ON a.id = t.address
             WHERE car.id = $1 AND car.user_id = $2
             ORDER BY t.created ASC",
            Self::TEAM_ADDRESS_COLUMNS
        );
        let rows: Vec<TeamAddressRow> = sqlx::query_as(&query)
            .bind(cook_and_run_id_filter)
            .bind(user_id_filter)
            .fetch_all(&self.pool)
            .await
            .map_err(AppError::DatabaseError)?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_team(
        &self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(Team, Address), AppError> {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1 AND car.id = $2 AND car.user_id = $3",
            Self::TEAM_ADDRESS_COLUMNS
        );
        let row: TeamAddressRow = sqlx::query_as(&query)
            .bind(id_filter)
            .bind(cook_and_run_id_filter)
            .bind(user_id_filter)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::TeamNotFound(
                    *id_filter,
                    user_id_filter.to_string(),
                    *cook_and_run_id_filter,
                ),
                other => AppError::DatabaseError(other),
            })?;

        Ok(row.into())
    }

    /// Self-service access for participants: lookup purely via the deeplink
    /// token, without knowing cook_and_run_id/user_id.
    #[tracing::instrument(skip(self))]
    pub async fn select_team_by_token(
        &self,
        access_token: &str,
    ) -> Result<(Team, Address), AppError> {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN address a ON a.id = t.address
             WHERE t.access_token = $1",
            Self::TEAM_ADDRESS_COLUMNS
        );
        let row: TeamAddressRow = sqlx::query_as(&query)
            .bind(access_token)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::TeamNotFoundByToken,
                other => AppError::DatabaseError(other),
            })?;

        Ok(row.into())
    }

    /// Edit deadline that applies to a team's self-service token, derived
    /// from its share configuration. `None` if there is no deadline
    /// configured (or no share config at all for that cook-and-run).
    #[tracing::instrument(skip(self))]
    pub async fn select_edit_deadline_by_token(
        &self,
        access_token: &str,
    ) -> Result<Option<DateTime<Utc>>, AppError> {
        sqlx::query_scalar(
            "SELECT s.edit_deadline
             FROM team t
             INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
             LEFT JOIN share s ON s.id = car.share_team_config
             WHERE t.access_token = $1",
        )
        .bind(access_token)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::TeamNotFoundByToken,
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self))]
    pub async fn select_team_by_id_unchecked(&self, id: &Uuid) -> Result<(Team, Address), AppError> {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1",
            Self::TEAM_ADDRESS_COLUMNS
        );
        let row: TeamAddressRow = sqlx::query_as(&query)
            .bind(id)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    AppError::TeamNotFound(*id, "NONE".to_string(), Uuid::nil())
                }
                other => AppError::DatabaseError(other),
            })?;
        Ok(row.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_team(
        &self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        // Before deleting, check whether the team is referenced by the plan.
        // No audit entry here — it would be cascade-deleted immediately
        // anyway (team_audit_log FK is ON DELETE CASCADE on team_id).
        if let Some(plan_data) =
            plan_staleness::plan_data_if_present(&mut tx, cook_and_run_id_filter).await?
        {
            if plan_staleness::plan_references_team(&plan_data, id_filter) {
                plan_staleness::mark_plan_stale(
                    &mut tx,
                    cook_and_run_id_filter,
                    &chrono::Utc::now(),
                )
                .await?;
            }
        }

        let affected = sqlx::query(
            "DELETE FROM team
             WHERE id = $1
               AND cook_and_run_id IN (
                   SELECT id FROM cook_and_run WHERE id = $2 AND user_id = $3
               )",
        )
        .bind(id_filter)
        .bind(cook_and_run_id_filter)
        .bind(user_id_filter)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::TeamNotFound(
                *id_filter,
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// Admin update (JWT). Status/token/verification are left untouched.
    /// Canceled team -> AppError::TeamCanceled, even for the admin.
    #[tracing::instrument(skip(self, data, address_data))]
    pub async fn update_team(
        &self,
        data: &Team,
        address_data: &Address,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        let before: Option<TeamBeforeUpdate> = sqlx::query_as(
            "SELECT t.id, t.cook_and_run_id, t.status, t.name, t.mail, t.phone, t.members, t.diets,
                    a.address_text, a.latitude, a.longitude
             FROM team t
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1
               AND (
                   t.cook_and_run_id IN (
                       SELECT id FROM cook_and_run WHERE id = $2 AND user_id = $3
                   )
                   OR t.created_by_user = $3
               )
             FOR UPDATE OF t",
        )
        .bind(data.id)
        .bind(data.cook_and_run_id)
        .bind(user_id_filter)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let Some(before) = before else {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::TeamNotFound(
                data.id,
                user_id_filter.to_string(),
                data.cook_and_run_id,
            ));
        };

        if before.status == TeamStatus::Canceled {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::TeamCanceled);
        }

        create_address(&mut *tx, address_data).await?;

        sqlx::query(
            "UPDATE team
             SET name = $1, edited = $2, address = $3, mail = $4,
                 phone = $5, members = $6, diets = $7
             WHERE id = $8",
        )
        .bind(&data.name)
        .bind(data.edited)
        .bind(data.address)
        .bind(&data.mail)
        .bind(&data.phone)
        .bind(data.members)
        .bind(&data.diets)
        .bind(data.id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let before_snapshot = before.snapshot();
        let after = TeamAuditSnapshot {
            name: data.name.clone(),
            mail: data.mail.clone(),
            phone: data.phone.clone(),
            members: data.members,
            diets: data.diets.clone(),
            address_text: address_data.address_text.clone(),
            latitude: address_data.latitude,
            longitude: address_data.longitude,
        };

        if before_snapshot != after {
            insert_audit_row(
                &mut tx,
                &data.id,
                AuditActorType::Admin,
                Some(user_id_filter),
                AuditAction::Updated,
                &diff_json(&before_snapshot, &after),
                &data.edited,
            )
            .await?;
        }

        let address_changed = before_snapshot.address_text != after.address_text
            || before_snapshot.latitude != after.latitude
            || before_snapshot.longitude != after.longitude;
        if address_changed {
            mark_plan_stale_if_referenced(
                &mut tx,
                &data.cook_and_run_id,
                &data.id,
                "address_changed",
                &data.edited,
            )
            .await?;
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// Self-service update (token auth). Checks review-trigger fields and,
    /// if needed, enqueues an admin notification in the same transaction as
    /// the audit entry.
    #[tracing::instrument(skip(self, data, address_data, review_trigger_fields, admin_notification))]
    pub async fn update_team_by_token(
        &self,
        data: &Team,
        address_data: &Address,
        access_token: &str,
        review_trigger_fields: &[TeamFields],
        admin_notification: Option<AdminNotificationTarget<'_>>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        let before: Option<TeamBeforeUpdate> = sqlx::query_as(
            "SELECT t.id, t.cook_and_run_id, t.status, t.name, t.mail, t.phone, t.members, t.diets,
                    a.address_text, a.latitude, a.longitude
             FROM team t
             INNER JOIN address a ON a.id = t.address
             WHERE t.access_token = $1
             FOR UPDATE OF t",
        )
        .bind(access_token)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let Some(before) = before else {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::TeamNotFoundByToken);
        };

        if before.status == TeamStatus::Canceled {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::TeamCanceled);
        }

        create_address(&mut *tx, address_data).await?;

        sqlx::query(
            "UPDATE team
             SET name = $1, edited = $2, address = $3, mail = $4,
                 phone = $5, members = $6, diets = $7
             WHERE id = $8",
        )
        .bind(&data.name)
        .bind(data.edited)
        .bind(data.address)
        .bind(&data.mail)
        .bind(&data.phone)
        .bind(data.members)
        .bind(&data.diets)
        .bind(before.id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let before_snapshot = before.snapshot();
        let after = TeamAuditSnapshot {
            name: data.name.clone(),
            mail: data.mail.clone(),
            phone: data.phone.clone(),
            members: data.members,
            diets: data.diets.clone(),
            address_text: address_data.address_text.clone(),
            latitude: address_data.latitude,
            longitude: address_data.longitude,
        };

        if before_snapshot != after {
            let mut changed_fields: Vec<TeamFields> = Vec::new();
            if before_snapshot.mail != after.mail {
                changed_fields.push(TeamFields::Mail);
            }
            if before_snapshot.phone != after.phone {
                changed_fields.push(TeamFields::Phone);
            }
            if before_snapshot.members != after.members {
                changed_fields.push(TeamFields::Members);
            }
            if before_snapshot.diets != after.diets {
                changed_fields.push(TeamFields::Diets);
            }

            let triggers_review = changed_fields
                .iter()
                .any(|f| review_trigger_fields.contains(f));

            let mut changes = diff_json(&before_snapshot, &after);
            if triggers_review {
                sqlx::query("UPDATE team SET status = 'review' WHERE id = $1")
                    .bind(before.id)
                    .execute(&mut *tx)
                    .await
                    .map_err(AppError::DatabaseError)?;

                if let serde_json::Value::Object(ref mut map) = changes {
                    map.insert("status_changed_to".to_string(), json!("review"));
                }

                if let Some(target) = admin_notification {
                    let reason_summary = changed_fields
                        .iter()
                        .map(field_display_name)
                        .collect::<Vec<_>>()
                        .join(", ");
                    let review_context = json!({
                        "kind": "review",
                        "team_name": data.name,
                        "cook_and_run_name": target.cook_and_run_name,
                        "admin_team_link_url": target.admin_team_link_url,
                        "reason_summary": format!("Changed fields: {reason_summary}"),
                    });
                    insert_email_outbox_tx(
                        &mut tx,
                        Some(before.id),
                        target.recipient_email,
                        EmailType::AdminNotification,
                        &review_context,
                        &data.edited,
                    )
                    .await?;
                }
            }

            insert_audit_row(
                &mut tx,
                &before.id,
                AuditActorType::Participant,
                None,
                AuditAction::Updated,
                &changes,
                &data.edited,
            )
            .await?;
        }

        let address_changed = before_snapshot.address_text != after.address_text
            || before_snapshot.latitude != after.latitude
            || before_snapshot.longitude != after.longitude;
        if address_changed {
            mark_plan_stale_if_referenced(
                &mut tx,
                &before.cook_and_run_id,
                &before.id,
                "address_changed",
                &data.edited,
            )
            .await?;
        }

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    /// Cancellation by the participant. Idempotent (no repeated audit entry
    /// / no repeated email on double invocation). Optionally enqueues an
    /// admin notification.
    #[tracing::instrument(skip(self, admin_notification))]
    pub async fn cancel_team_by_token(
        &self,
        access_token: &str,
        reason: Option<&str>,
        time: &DateTime<Utc>,
        admin_notification: Option<AdminNotificationTarget<'_>>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        let before: Option<TeamCancelLookup> = sqlx::query_as(
            "SELECT id, cook_and_run_id, status, name FROM team WHERE access_token = $1 FOR UPDATE",
        )
        .bind(access_token)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let Some((team_id, cook_and_run_id, previous_status, team_name)) = before.map(Into::into)
        else {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::TeamNotFoundByToken);
        };

        if previous_status == TeamStatus::Canceled {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Ok(());
        }

        sqlx::query(
            "UPDATE team SET status = 'canceled', canceled_at = $1, cancel_reason = $2, edited = $1
             WHERE id = $3",
        )
        .bind(time)
        .bind(reason)
        .bind(team_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?;

        let changes = json!({
            "previous_status": format!("{:?}", previous_status),
            "reason": reason,
        });
        insert_audit_row(
            &mut tx,
            &team_id,
            AuditActorType::Participant,
            None,
            AuditAction::Canceled,
            &changes,
            time,
        )
        .await?;

        if let Some(target) = admin_notification {
            let notification_context = json!({
                "kind": "cancellation",
                "team_name": team_name,
                "cook_and_run_name": target.cook_and_run_name,
                "admin_team_link_url": target.admin_team_link_url,
                "reason_summary": reason.unwrap_or("No reason provided"),
            });
            insert_email_outbox_tx(
                &mut tx,
                Some(team_id),
                target.recipient_email,
                EmailType::AdminNotification,
                &notification_context,
                time,
            )
            .await?;
        }

        mark_plan_stale_if_referenced(&mut tx, &cook_and_run_id, &team_id, "team_canceled", time)
            .await?;

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn verify_team_email_by_token(
        &self,
        access_token: &str,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError> {
        let affected =
            sqlx::query("UPDATE team SET email_verified_at = $1 WHERE access_token = $2")
                .bind(time)
                .bind(access_token)
                .execute(&self.pool)
                .await
                .map_err(AppError::DatabaseError)?
                .rows_affected();

        if affected == 0 {
            return Err(AppError::TeamNotFoundByToken);
        }
        Ok(())
    }

    /// Increments the resend counter and returns the new value. The limit
    /// (max. 3 for participants) is enforced by the domain layer.
    #[tracing::instrument(skip(self))]
    pub async fn increment_verification_resend_count_by_token(
        &self,
        access_token: &str,
    ) -> Result<i32, AppError> {
        sqlx::query_scalar(
            "UPDATE team
             SET verification_resend_count = verification_resend_count + 1
             WHERE access_token = $1
             RETURNING verification_resend_count",
        )
        .bind(access_token)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::TeamNotFoundByToken,
            other => AppError::DatabaseError(other),
        })
    }

    /// Enqueues a route-update email and records the new route hash
    /// atomically — prevents a hash from being marked "sent" even though
    /// the enqueue failed (or vice versa).
    #[tracing::instrument(skip(self, context))]
    pub async fn enqueue_route_update_and_record_hash(
        &self,
        team_id: &Uuid,
        recipient_email: &str,
        new_hash: &str,
        context: &serde_json::Value,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        insert_email_outbox_tx(
            &mut tx,
            Some(*team_id),
            recipient_email,
            EmailType::RouteUpdate,
            context,
            time,
        )
        .await?;

        sqlx::query("UPDATE team SET last_route_hash = $1 WHERE id = $2")
            .bind(new_hash)
            .bind(team_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::DatabaseError)?;

        tx.commit().await.map_err(AppError::DatabaseError)?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct TeamAddressRow {
    id: Uuid,
    cook_and_run_id: Uuid,
    created_by_user: Option<String>,
    name: String,
    created: DateTime<Utc>,
    edited: DateTime<Utc>,
    address: Uuid,
    mail: Option<String>,
    phone: Option<String>,
    members: Option<i32>,
    diets: Option<String>,
    status: TeamStatus,
    canceled_at: Option<DateTime<Utc>>,
    cancel_reason: Option<String>,
    access_token: String,
    email_verified_at: Option<DateTime<Utc>>,
    verification_resend_count: i32,
    last_route_hash: Option<String>,
    a_id: Uuid,
    a_address_text: String,
    a_latitude: f64,
    a_longitude: f64,
}

impl From<TeamAddressRow> for (Team, Address) {
    fn from(row: TeamAddressRow) -> Self {
        (
            Team {
                id: row.id,
                cook_and_run_id: row.cook_and_run_id,
                created_by_user: row.created_by_user,
                name: row.name,
                created: row.created,
                edited: row.edited,
                address: row.address,
                mail: row.mail,
                phone: row.phone,
                members: row.members,
                diets: row.diets,
                status: row.status,
                canceled_at: row.canceled_at,
                cancel_reason: row.cancel_reason,
                access_token: row.access_token,
                email_verified_at: row.email_verified_at,
                verification_resend_count: row.verification_resend_count,
                last_route_hash: row.last_route_hash,
            },
            Address {
                id: row.a_id,
                address_text: row.a_address_text,
                latitude: row.a_latitude,
                longitude: row.a_longitude,
            },
        )
    }
}

#[derive(sqlx::FromRow)]
struct TeamBeforeUpdate {
    id: Uuid,
    cook_and_run_id: Uuid,
    status: TeamStatus,
    name: String,
    mail: Option<String>,
    phone: Option<String>,
    members: Option<i32>,
    diets: Option<String>,
    address_text: String,
    latitude: f64,
    longitude: f64,
}

impl TeamBeforeUpdate {
    fn snapshot(&self) -> TeamAuditSnapshot {
        TeamAuditSnapshot {
            name: self.name.clone(),
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members,
            diets: self.diets.clone(),
            address_text: self.address_text.clone(),
            latitude: self.latitude,
            longitude: self.longitude,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TeamCancelLookup {
    id: Uuid,
    cook_and_run_id: Uuid,
    status: TeamStatus,
    name: String,
}

impl From<TeamCancelLookup> for (Uuid, Uuid, TeamStatus, String) {
    fn from(row: TeamCancelLookup) -> Self {
        (row.id, row.cook_and_run_id, row.status, row.name)
    }
}