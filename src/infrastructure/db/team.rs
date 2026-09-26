use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::domain::address::Address;
use crate::domain::team::{Team, TeamStatus};
use crate::error::AppError;
use crate::infrastructure::db::address::AddressEntity;

pub struct TeamRepository;

#[derive(Debug, Clone, FromRow)]
struct TeamEntity {
    id: Uuid,
    project_id: Uuid,
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
}

impl TeamEntity {
    fn from_domain(team: &Team) -> Self {
        TeamEntity {
            id: team.id,
            project_id: team.project_id,
            created_by_user: team.created_by_user.clone(),
            name: team.name.clone(),
            created: team.created,
            edited: team.edited,
            address: team.address.id,
            mail: team.mail.clone(),
            phone: team.phone.clone(),
            members: team.members.map(|m| m as i32),
            diets: team.diets.clone(),
            status: team.status,
            canceled_at: team.canceled_at,
            cancel_reason: team.cancel_reason.clone(),
            access_token: team.access_token.clone(),
            email_verified_at: team.email_verified_at,
            verification_resend_count: team.verification_resend_count as i32,
            last_route_hash: team.last_route_hash.clone(),
        }
    }

    fn to_domain(&self, address: &AddressEntity) -> Team {
        Team {
            id: self.id,
            project_id: self.project_id,
            created_by_user: self.created_by_user.clone(),
            name: self.name.clone(),
            created: self.created,
            edited: self.edited,
            address: address.to_domain(),
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members.map(|m| m as u32),
            diets: self.diets.clone(),
            status: self.status,
            canceled_at: self.canceled_at,
            cancel_reason: self.cancel_reason.clone(),
            access_token: self.access_token.clone(),
            email_verified_at: self.email_verified_at,
            verification_resend_count: self.verification_resend_count as u32,
            last_route_hash: self.last_route_hash.clone(),
            note_list: Vec::new(),
        }
    }
}

const TEAM_COLUMNS: &str = "
    id, project_id, created_by_user, name, created, edited,
    address, mail, phone, members, diets,
    status, canceled_at, cancel_reason, access_token,
    email_verified_at, verification_resend_count, last_route_hash";

impl TeamRepository {
    /// Idempotent on a unique-constraint clash (23505), matching the other
    /// `insert` methods in this module.
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn insert<'e, E>(&self, executor: E, data: &Team) -> Result<(), AppError>
    where
        E: sqlx::Executor<'e, Database = sqlx::Postgres>,
    {
        let team = TeamEntity::from_domain(data);

        let result = sqlx::query(
            "INSERT INTO team
                (id, project_id, created_by_user, name, created, edited,
                 address, mail, phone, members, diets,
                 status, canceled_at, cancel_reason, access_token,
                 email_verified_at, verification_resend_count, last_route_hash)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                     $12, $13, $14, $15, $16, $17, $18)",
        )
        .bind(team.id)
        .bind(team.project_id)
        .bind(&team.created_by_user)
        .bind(&team.name)
        .bind(team.created)
        .bind(team.edited)
        .bind(team.address)
        .bind(&team.mail)
        .bind(&team.phone)
        .bind(team.members)
        .bind(&team.diets)
        .bind(team.status)
        .bind(team.canceled_at)
        .bind(&team.cancel_reason)
        .bind(&team.access_token)
        .bind(team.email_verified_at)
        .bind(team.verification_resend_count)
        .bind(&team.last_route_hash)
        .execute(executor)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
                Ok(())
            }
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn count_for_project<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
    ) -> Result<i64, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM team t
             INNER JOIN project car ON car.id = t.project_id
             WHERE car.id = $1",
        )
        .bind(project_id_filter)
        .fetch_one(executor)
        .await
        .map_err(AppError::DatabaseError)
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_all_for_project<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<Team>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN project car ON car.id = t.project_id
             WHERE car.id = $1 AND car.user_id = $2
             ORDER BY t.created ASC",
            TEAM_COLUMNS
        );

        sqlx::query_as::<_, TeamEntity>(&query)
            .bind(project_id_filter)
            .bind(user_id_filter)
            .fetch_all(executor)
            .await
            .map_err(AppError::DatabaseError)
            .map(|rows| rows.iter().map(TeamEntity::to_domain).collect())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        project_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Team, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN project car ON car.id = t.project_id
             WHERE t.id = $1 AND car.id = $2 AND car.user_id = $3",
            TEAM_COLUMNS
        );

        sqlx::query_as::<_, TeamEntity>(&query)
            .bind(id_filter)
            .bind(project_id_filter)
            .bind(user_id_filter)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::TeamNotFound(
                    *id_filter,
                    user_id_filter.to_string(),
                    *project_id_filter,
                ),
                other => AppError::DatabaseError(other),
            })
            .map(|row| row.to_domain())
    }

    /// Self-service access for participants: lookup purely via the
    /// deeplink token, without knowing project_id/user_id.
    #[tracing::instrument(skip(self, executor))]
    pub async fn select_by_token<'e, E>(
        &self,
        executor: E,
        access_token: &str,
    ) -> Result<Team, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!(
            "SELECT {} FROM team t WHERE t.access_token = $1",
            TEAM_COLUMNS
        );

        sqlx::query_as::<_, TeamEntity>(&query)
            .bind(access_token)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::TeamNotFoundByToken,
                other => AppError::DatabaseError(other),
            })
            .map(|row| row.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_by_id_unchecked<'e, E>(
        &self,
        executor: E,
        id: &Uuid,
    ) -> Result<Team, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!("SELECT {} FROM team t WHERE t.id = $1", TEAM_COLUMNS);

        sqlx::query_as::<_, TeamEntity>(&query)
            .bind(id)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    AppError::TeamNotFound(*id, "NONE".to_string(), Uuid::nil())
                }
                other => AppError::DatabaseError(other),
            })
            .map(|row| row.to_domain())
    }

    /// Row-locking variant of `select`, for callers that need to read the
    /// current state and then update it in the same transaction (e.g. an
    /// audit-diffed update). Matches admin ownership OR
    /// `created_by_user`, as the original admin-update lookup did.
    #[tracing::instrument(skip(self, executor))]
    pub async fn select_locked_for_admin_update<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        project_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Option<Team>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!(
            "SELECT {}
             FROM team t
             WHERE t.id = $1
               AND (
                   t.project_id IN (
                       SELECT id FROM project WHERE id = $2 AND user_id = $3
                   )
                   OR t.created_by_user = $3
               )
             FOR UPDATE OF t",
            TEAM_COLUMNS
        );

        sqlx::query_as::<_, TeamEntity>(&query)
            .bind(id_filter)
            .bind(project_id_filter)
            .bind(user_id_filter)
            .fetch_optional(executor)
            .await
            .map_err(AppError::DatabaseError)
            .map(|row| row.map(|r| r.to_domain()))
    }

    /// Row-locking lookup by token, for the self-service update path.
    #[tracing::instrument(skip(self, executor))]
    pub async fn select_locked_by_token<'e, E>(
        &self,
        executor: E,
        access_token: &str,
    ) -> Result<Option<Team>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!(
            "SELECT {} FROM team t WHERE t.access_token = $1 FOR UPDATE OF t",
            TEAM_COLUMNS
        );

        sqlx::query_as::<_, TeamEntity>(&query)
            .bind(access_token)
            .fetch_optional(executor)
            .await
            .map_err(AppError::DatabaseError)
            .map(|row| row.map(|r| r.to_domain()))
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn delete<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        project_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            "DELETE FROM team
             WHERE id = $1
               AND project_id IN (
                   SELECT id FROM project WHERE id = $2 AND user_id = $3
               )",
        )
        .bind(id_filter)
        .bind(project_id_filter)
        .bind(user_id_filter)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::TeamNotFound(
                *id_filter,
                user_id_filter.to_string(),
                *project_id_filter,
            ));
        }
        Ok(())
    }

    /// Updates the mutable participant/contact fields plus the address FK.
    /// Does not touch `status` — use `update_status_review` /
    /// `update_cancel` for status transitions.
    #[tracing::instrument(skip(self, executor, data))]
    pub async fn update_fields<'e, E>(&self, executor: E, data: &Team) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let team = TeamEntity::from_domain(data);

        let affected = sqlx::query(
            "UPDATE team
             SET name = $1, edited = $2, address = $3, mail = $4,
                 phone = $5, members = $6, diets = $7
             WHERE id = $8",
        )
        .bind(&team.name)
        .bind(team.edited)
        .bind(team.address)
        .bind(&team.mail)
        .bind(&team.phone)
        .bind(team.members)
        .bind(&team.diets)
        .bind(team.id)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::TeamNotFound(
                data.id,
                data.created_by_user.clone().unwrap_or_default(),
                data.project_id,
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn update_status_review<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query("UPDATE team SET status = 'review' WHERE id = $1")
            .bind(id_filter)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::TeamNotFound(
                *id_filter,
                String::new(),
                Uuid::nil(),
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn update_cancel<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        time: &DateTime<Utc>,
        reason: Option<&str>,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            "UPDATE team SET status = 'canceled', canceled_at = $1, cancel_reason = $2, edited = $1
             WHERE id = $3",
        )
        .bind(time)
        .bind(reason)
        .bind(id_filter)
        .execute(executor)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::TeamNotFoundByToken);
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn verify_email_by_token<'e, E>(
        &self,
        executor: E,
        access_token: &str,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected =
            sqlx::query("UPDATE team SET email_verified_at = $1 WHERE access_token = $2")
                .bind(time)
                .bind(access_token)
                .execute(executor)
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
    #[tracing::instrument(skip(self, executor))]
    pub async fn increment_resend_count_by_token<'e, E>(
        &self,
        executor: E,
        access_token: &str,
    ) -> Result<i32, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_scalar(
            "UPDATE team
             SET verification_resend_count = verification_resend_count + 1
             WHERE access_token = $1
             RETURNING verification_resend_count",
        )
        .bind(access_token)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::TeamNotFoundByToken,
            other => AppError::DatabaseError(other),
        })
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn update_last_route_hash<'e, E>(
        &self,
        executor: E,
        team_id: &Uuid,
        new_hash: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query("UPDATE team SET last_route_hash = $1 WHERE id = $2")
            .bind(new_hash)
            .bind(team_id)
            .execute(executor)
            .await
            .map_err(AppError::DatabaseError)?;
        Ok(())
    }
}

// ============================================================
// Read-only join projections
// ============================================================
// These return (Team, Address) pairs the way most callers actually need
// them. They stay as plain, generic-executor functions outside
// `TeamRepository` (same idea as `email_context.rs`): a join across two
// entities for read convenience is not entity persistence, so it does not
// belong inside either `TeamRepository` or `AddressRepository`. Callers
// that don't need the address can use the plain `TeamRepository` methods
// above and avoid the join entirely.

#[derive(FromRow)]
struct TeamAddressRow {
    #[sqlx(flatten)]
    team: TeamEntity,
    #[sqlx(flatten)]
    address: AddressEntity,
}

const TEAM_ADDRESS_COLUMNS: &str = "
    t.id, t.project_id, t.created_by_user, t.name, t.created, t.edited,
    t.address, t.mail, t.phone, t.members, t.diets,
    t.status, t.canceled_at, t.cancel_reason, t.access_token,
    t.email_verified_at, t.verification_resend_count, t.last_route_hash,
    a.id AS a_id, a.address_text AS a_address_text,
    a.latitude AS a_latitude, a.longitude AS a_longitude";

impl From<TeamAddressRow> for (Team, Address) {
    fn from(row: TeamAddressRow) -> Self {
        let address = AddressEntity {
            id: row.a_id,
            address_text: row.a_address_text,
            latitude: row.a_latitude,
            longitude: row.a_longitude,
        }
        .to_domain();

        let mut team = row.team.to_domain(&row.address);
        team.address = address.clone();

        (team, address)
    }
}

#[tracing::instrument(skip(executor))]
pub async fn select_all_teams_with_address<'e, E>(
    executor: E,
    project_id_filter: &Uuid,
    user_id_filter: &str,
) -> Result<Vec<(Team, Address)>, AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    let query = format!(
        "SELECT {}
         FROM team t
         INNER JOIN project car ON car.id = t.project_id
         INNER JOIN address a ON a.id = t.address
         WHERE car.id = $1 AND car.user_id = $2
         ORDER BY t.created ASC",
        TEAM_ADDRESS_COLUMNS
    );
    let rows: Vec<TeamAddressRow> = sqlx::query_as(&query)
        .bind(project_id_filter)
        .bind(user_id_filter)
        .fetch_all(executor)
        .await
        .map_err(AppError::DatabaseError)?;

    Ok(rows.into_iter().map(Into::into).collect())
}

#[tracing::instrument(skip(executor))]
pub async fn select_team_with_address<'e, E>(
    executor: E,
    id_filter: &Uuid,
    project_id_filter: &Uuid,
    user_id_filter: &str,
) -> Result<(Team, Address), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    let query = format!(
        "SELECT {}
         FROM team t
         INNER JOIN project car ON car.id = t.project_id
         INNER JOIN address a ON a.id = t.address
         WHERE t.id = $1 AND car.id = $2 AND car.user_id = $3",
        TEAM_ADDRESS_COLUMNS
    );
    let row: TeamAddressRow = sqlx::query_as(&query)
        .bind(id_filter)
        .bind(project_id_filter)
        .bind(user_id_filter)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                AppError::TeamNotFound(*id_filter, user_id_filter.to_string(), *project_id_filter)
            }
            other => AppError::DatabaseError(other),
        })?;

    Ok(row.into())
}

/// Self-service access for participants: lookup purely via the deeplink
/// token, without knowing project_id/user_id.
#[tracing::instrument(skip(executor))]
pub async fn select_team_with_address_by_token<'e, E>(
    executor: E,
    access_token: &str,
) -> Result<(Team, Address), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    let query = format!(
        "SELECT {}
         FROM team t
         INNER JOIN address a ON a.id = t.address
         WHERE t.access_token = $1",
        TEAM_ADDRESS_COLUMNS
    );
    let row: TeamAddressRow = sqlx::query_as(&query)
        .bind(access_token)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => AppError::TeamNotFoundByToken,
            other => AppError::DatabaseError(other),
        })?;

    Ok(row.into())
}

#[tracing::instrument(skip(executor))]
pub async fn select_team_with_address_by_id_unchecked<'e, E>(
    executor: E,
    id: &Uuid,
) -> Result<(Team, Address), AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    let query = format!(
        "SELECT {}
         FROM team t
         INNER JOIN address a ON a.id = t.address
         WHERE t.id = $1",
        TEAM_ADDRESS_COLUMNS
    );
    let row: TeamAddressRow = sqlx::query_as(&query)
        .bind(id)
        .fetch_one(executor)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => {
                AppError::TeamNotFound(*id, "NONE".to_string(), Uuid::nil())
            }
            other => AppError::DatabaseError(other),
        })?;

    Ok(row.into())
}

/// Edit deadline that applies to a team's self-service token, derived from
/// its project's share configuration. `None` if there is no deadline
/// configured (or no share config at all for that project).
#[tracing::instrument(skip(executor))]
pub async fn select_edit_deadline_by_token<'e, E>(
    executor: E,
    access_token: &str,
) -> Result<Option<DateTime<Utc>>, AppError>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_scalar(
        "SELECT s.edit_deadline
         FROM team t
         INNER JOIN project car ON car.id = t.project_id
         LEFT JOIN share s ON s.id = car.share_team_config
         WHERE t.access_token = $1",
    )
    .bind(access_token)
    .fetch_one(executor)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => AppError::TeamNotFoundByToken,
        other => AppError::DatabaseError(other),
    })
}
