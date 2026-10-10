use chrono::{DateTime, Utc};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::domain::address::Address;
use crate::domain::team::{Team, TeamSortOption, TeamStatus};
use crate::error::AppError;
use crate::infrastructure::db::address::AddressEntity;

pub struct TeamRepository;

/// DB-layer mirror of `domain::team::TeamStatus`. Kept separate so the
/// domain type stays free of sqlx; this is the only enum sqlx needs to
/// know how to encode/decode against the Postgres `team_status` type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "team_status", rename_all = "snake_case")]
enum TeamStatusEntity {
    Active,
    Review,
    Canceled,
}

impl TeamStatusEntity {
    fn from_domain(status: TeamStatus) -> Self {
        match status {
            TeamStatus::Active => TeamStatusEntity::Active,
            TeamStatus::Review => TeamStatusEntity::Review,
            TeamStatus::Canceled => TeamStatusEntity::Canceled,
        }
    }

    fn to_domain(&self) -> TeamStatus {
        match self {
            TeamStatusEntity::Active => TeamStatus::Active,
            TeamStatusEntity::Review => TeamStatus::Review,
            TeamStatusEntity::Canceled => TeamStatus::Canceled,
        }
    }
}

#[derive(Debug)]
enum TeamSortOptionEntity {
    NameAsc,
    NameDesc,
    CreatedAsc,
    CreatedDesc,
}

impl TeamSortOptionEntity {
    fn from_domain(team_sort_option: TeamSortOption) -> Self {
        match team_sort_option {
            TeamSortOption::CreatedAsc => TeamSortOptionEntity::CreatedAsc,
            TeamSortOption::CreatedDesc => TeamSortOptionEntity::CreatedDesc,
            TeamSortOption::NameAsc => TeamSortOptionEntity::NameAsc,
            TeamSortOption::NameDesc => TeamSortOptionEntity::NameDesc,
        }
    }

    /// SQL `ORDER BY` fragment for this sort option. The set of variants is
    /// a closed enum, so interpolating the value into the query is
    /// injection-safe.
    fn order_by_clause(&self) -> &'static str {
        match self {
            TeamSortOptionEntity::NameAsc => "t.name ASC",
            TeamSortOptionEntity::NameDesc => "t.name DESC",
            TeamSortOptionEntity::CreatedAsc => "t.created ASC",
            TeamSortOptionEntity::CreatedDesc => "t.created DESC",
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct TeamMetaEntity {
    id: Uuid,
    project_id: Uuid,
    created_by_user: Option<String>,
    name: String,
    created: DateTime<Utc>,
    edited: DateTime<Utc>,
    mail: Option<String>,
    phone: Option<String>,
    members: Option<i32>,
    diets: Option<String>,
    status: TeamStatusEntity,
    canceled_at: Option<DateTime<Utc>>,
    email_verified_at: Option<DateTime<Utc>>,
    verification_resend_count: i32,
}

impl TeamMetaEntity {
    fn to_domain(&self) -> Team {
        Team {
            id: self.id,
            project_id: self.project_id,
            created_by_user: self.created_by_user.clone(),
            name: self.name.clone(),
            created: self.created,
            edited: self.edited,
            address: Address::default(),
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members.map(|m| m as u32),
            diets: self.diets.clone(),
            status: self.status.to_domain(),
            canceled_at: self.canceled_at,
            cancel_reason: None,
            access_token: "".to_string(),
            email_verified_at: self.email_verified_at,
            verification_resend_count: self.verification_resend_count as u32,
            last_route_hash: None,
        }
    }
}

#[derive(Debug, Clone, FromRow)]
struct TeamEntity {
    // Im JOIN mit `address` gibt es zwei Spalten namens `id`. Die Team-ID
    // wird deshalb in den Abfragen als `team_id` selektiert (siehe
    // TEAM_COLUMS / TEAM_ADDRESS_COLUMNS).
    #[sqlx(rename = "team_id")]
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
    status: TeamStatusEntity,
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
            status: TeamStatusEntity::from_domain(team.status),
            canceled_at: team.canceled_at,
            cancel_reason: team.cancel_reason.clone(),
            access_token: team.access_token.clone(),
            email_verified_at: team.email_verified_at,
            verification_resend_count: team.verification_resend_count as i32,
            last_route_hash: team.last_route_hash.clone(),
        }
    }

    fn to_domain(&self) -> Team {
        Team {
            id: self.id,
            project_id: self.project_id,
            created_by_user: self.created_by_user.clone(),
            name: self.name.clone(),
            created: self.created,
            edited: self.edited,
            address: Address::default(),
            mail: self.mail.clone(),
            phone: self.phone.clone(),
            members: self.members.map(|m| m as u32),
            diets: self.diets.clone(),
            status: self.status.to_domain(),
            canceled_at: self.canceled_at,
            cancel_reason: self.cancel_reason.clone(),
            access_token: self.access_token.clone(),
            email_verified_at: self.email_verified_at,
            verification_resend_count: self.verification_resend_count as u32,
            last_route_hash: self.last_route_hash.clone(),
        }
    }
}

#[derive(FromRow)]
struct TeamWithAddressRow {
    #[sqlx(flatten)]
    team: TeamEntity,
    #[sqlx(flatten)]
    address: AddressEntity,
}

impl TeamWithAddressRow {
    fn to_domain(&self) -> Team {
        Team {
            id: self.team.id,
            project_id: self.team.project_id,
            created_by_user: self.team.created_by_user.clone(),
            name: self.team.name.clone(),
            created: self.team.created,
            edited: self.team.edited,
            address: self.address.to_domain(),
            mail: self.team.mail.clone(),
            phone: self.team.phone.clone(),
            members: self.team.members.map(|m| m as u32),
            diets: self.team.diets.clone(),
            status: self.team.status.to_domain(),
            canceled_at: self.team.canceled_at,
            cancel_reason: self.team.cancel_reason.clone(),
            access_token: self.team.access_token.clone(),
            email_verified_at: self.team.email_verified_at,
            verification_resend_count: self.team.verification_resend_count as u32,
            last_route_hash: self.team.last_route_hash.clone(),
        }
    }
}

const TEAM_COLUMS: &str = "
    t.id AS team_id, t.project_id, t.created_by_user, t.name, t.created, t.edited,
    t.address, t.mail, t.phone, t.members, t.diets,
    t.status, t.canceled_at, t.cancel_reason, t.access_token,
    t.email_verified_at, t.verification_resend_count, t.last_route_hash
";

const TEAM_ADDRESS_COLUMNS: &str = "
    t.id AS team_id, t.project_id, t.created_by_user, t.name, t.created, t.edited,
    t.address, t.mail, t.phone, t.members, t.diets,
    t.status, t.canceled_at, t.cancel_reason, t.access_token,
    t.email_verified_at, t.verification_resend_count, t.last_route_hash,
    a.id, a.address_text, a.latitude, a.longitude";

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
            Err(e) => Err(AppError::from(e)),
        }
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn count_for_project<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
    ) -> Result<u8, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"
        SELECT COUNT(*) FROM team t
        INNER JOIN project car ON car.id = t.project_id
        WHERE car.id = $1
        "#,
        )
        .bind(project_id_filter)
        .fetch_one(executor)
        .await
        .map_err(AppError::from)
        .and_then(|count| Ok(count as u8))
    }

    /// Backs `team::get_list` (admin team list in `get.rs`).
    #[tracing::instrument(skip(self, executor))]
    pub async fn select_page_meta_for_project<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
        user_id_filter: &str,
        limit: u8,
        offset: u8,
        team_sort_option: Option<TeamSortOption>,
    ) -> Result<Vec<Team>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let sort = team_sort_option.map_or(
            TeamSortOptionEntity::CreatedAsc,
            TeamSortOptionEntity::from_domain,
        );

        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN project car ON car.id = t.project_id
             INNER JOIN address a ON a.id = t.address
             WHERE car.id = $1 AND car.user_id = $2
             ORDER BY {}
             LIMIT $3 OFFSET $4",
            TEAM_ADDRESS_COLUMNS,
            sort.order_by_clause()
        );

        let rows: Vec<TeamWithAddressRow> = sqlx::query_as(&query)
            .bind(project_id_filter)
            .bind(user_id_filter)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(executor)
            .await
            .map_err(AppError::from)?;

        Ok(rows.iter().map(TeamWithAddressRow::to_domain).collect())
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
             INNER JOIN address a ON a.id = t.address
             WHERE car.id = $1 AND car.user_id = $2
             ORDER BY t.created ASC",
            TEAM_ADDRESS_COLUMNS
        );

        let rows: Vec<TeamWithAddressRow> = sqlx::query_as(&query)
            .bind(project_id_filter)
            .bind(user_id_filter)
            .fetch_all(executor)
            .await
            .map_err(AppError::from)?;

        Ok(rows.iter().map(TeamWithAddressRow::to_domain).collect())
    }

    /// Backs `team::get` (admin single-team lookup in `get.rs`).
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
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1 AND car.id = $2 AND car.user_id = $3",
            TEAM_ADDRESS_COLUMNS
        );

        let row: TeamWithAddressRow = sqlx::query_as(&query)
            .bind(id_filter)
            .bind(project_id_filter)
            .bind(user_id_filter)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::team_not_found(
                    *id_filter,
                    user_id_filter.to_string(),
                    *project_id_filter,
                ),
                other => AppError::from(other),
            })?;

        Ok(row.to_domain())
    }

    /// Self-service access for participants: lookup purely via the
    /// deeplink token, without knowing project_id/user_id. Backs
    /// `team::get_by_token_with_deadline` in `get.rs`, alongside
    /// `select_edit_deadline_by_token` below.
    #[tracing::instrument(skip(self, executor))]
    pub async fn select_by_token<'e, E>(
        &self,
        executor: E,
        project_id: &Uuid,
        team_id: &Uuid,
        access_token: &str,
    ) -> Result<Team, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1 AND t.project_id = $2 AND t.access_token = $3",
            TEAM_ADDRESS_COLUMNS
        );

        let row: TeamWithAddressRow = sqlx::query_as(&query)
            .bind(team_id)
            .bind(project_id)
            .bind(access_token)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => AppError::team_not_found_by_token(),
                other => AppError::from(other),
            })?;

        Ok(row.to_domain())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_by_id_unsafe<'e, E>(
        &self,
        executor: E,
        id: &Uuid,
    ) -> Result<Team, AppError>
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

        let row: TeamWithAddressRow = sqlx::query_as(&query)
            .bind(id)
            .fetch_one(executor)
            .await
            .map_err(|e| match e {
                sqlx::Error::RowNotFound => {
                    AppError::TeamNotFound(*id, "NONE".to_string(), Uuid::nil())
                }
                other => AppError::from(other),
            })?;

        Ok(row.to_domain())
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
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1
               AND (
                   t.project_id IN (
                       SELECT id FROM project WHERE id = $2 AND user_id = $3
                   )
                   OR t.created_by_user = $3
               )
             FOR UPDATE OF t",
            TEAM_ADDRESS_COLUMNS
        );

        let row: Option<TeamWithAddressRow> = sqlx::query_as(&query)
            .bind(id_filter)
            .bind(project_id_filter)
            .bind(user_id_filter)
            .fetch_optional(executor)
            .await
            .map_err(AppError::from)?;

        Ok(row.map(|r| r.to_domain()))
    }

    /// Row-locking lookup by token, for the self-service update path.
    #[tracing::instrument(skip(self, executor))]
    pub async fn select_locked_by_token<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        project_id_filter: &Uuid,
        access_token: &str,
    ) -> Result<Option<Team>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let query = format!(
            "SELECT {}
             FROM team t
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1 AND t.access_token = $2  AND (
                   t.project_id IN (
                       SELECT id FROM project WHERE id = $3
                   )
               )
             FOR UPDATE OF t",
            TEAM_ADDRESS_COLUMNS
        );

        let row: Option<TeamWithAddressRow> = sqlx::query_as(&query)
            .bind(id_filter)
            .bind(access_token)
            .bind(project_id_filter)
            .fetch_optional(executor)
            .await
            .map_err(AppError::from)?;

        Ok(row.map(|r| r.to_domain()))
    }

    #[tracing::instrument(skip(self, executor, team_ids))]
    pub async fn select_by_ids_for_project<'e, E>(
        &self,
        executor: E,
        project_id_filter: &Uuid,
        team_ids: &Vec<Uuid>,
    ) -> Result<Vec<Team>, AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        // `&[Uuid]` binds to a Postgres `uuid[]`, matched against `= ANY(...)`,
        // so an empty slice yields an empty result set without a special case.
        let query = format!(
            "SELECT {}
             FROM team t
             WHERE t.project_id = $1 AND t.id = ANY($2)",
            TEAM_COLUMS
        );

        let rows: Vec<TeamEntity> = sqlx::query_as(&query)
            .bind(project_id_filter)
            .bind(team_ids)
            .fetch_all(executor)
            .await
            .map_err(AppError::from)?;

        Ok(rows.iter().map(TeamEntity::to_domain).collect())
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
        .map_err(AppError::from)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::team_not_found(
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
                 phone = $5, members = $6, diets = $7, status = $8
             WHERE id = $9",
        )
        .bind(&team.name)
        .bind(team.edited)
        .bind(team.address)
        .bind(&team.mail)
        .bind(&team.phone)
        .bind(team.members)
        .bind(&team.diets)
        .bind(&team.status)
        .bind(team.id)
        .execute(executor)
        .await
        .map_err(AppError::from)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::team_not_found(
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
            .map_err(AppError::from)?
            .rows_affected();

        if affected == 0 {
            return Err(AppError::team_not_found(
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
        .map_err(AppError::from)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::team_not_found_by_token());
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn verify_email_by_token<'e, E>(
        &self,
        executor: E,
        project_id: &Uuid,
        team_id: &Uuid,
        access_token: &str,
        time: &DateTime<Utc>,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let affected = sqlx::query(
            "UPDATE team SET email_verified_at = $1
             WHERE id = $2 AND project_id = $3 AND access_token = $4",
        )
        .bind(time)
        .bind(team_id)
        .bind(project_id)
        .bind(access_token)
        .execute(executor)
        .await
        .map_err(AppError::from)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::team_not_found_by_token());
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn increment_resend_count_by_token<'e, E>(
        &self,
        executor: E,
        team_id: &Uuid,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        sqlx::query(
            "UPDATE team
             SET verification_resend_count = verification_resend_count + 1
             WHERE id = $1",
        )
        .bind(team_id)
        .execute(executor)
        .await?;
        Ok(())
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
            .map_err(AppError::from)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, executor))]
    pub async fn select_to_check_existinse<'e, E>(
        &self,
        executor: E,
        id_filter: &Uuid,
        project_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError>
    where
        E: sqlx::PgExecutor<'e>,
    {
        let team = sqlx::query(
            r#"
        SELECT 1
        FROM team t
        WHERE t.id = $1
          AND t.project_id = $2
          AND EXISTS (
              SELECT 1 FROM project
              WHERE id = $2 AND user_id = $3
          )
        FOR UPDATE OF t
        "#,
        )
        .bind(id_filter)
        .bind(project_id_filter)
        .bind(user_id_filter)
        .fetch_optional(executor)
        .await
        .map_err(AppError::from)?;

        if team.is_none() {
            return Err(AppError::team_not_found(
                *id_filter,
                user_id_filter.to_string(),
                *project_id_filter,
            ));
        }

        Ok(())
    }
}
