use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::address::create_address;
use crate::db::models::{Address, Team};
use crate::error::AppError;

impl super::Database {
    #[tracing::instrument(skip(self, data, address_data))]
    pub async fn create_team(&self, data: &Team, address_data: &Address) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        create_address(&mut *tx, address_data).await?;

        let result = sqlx::query(
            "INSERT INTO team
                (id, cook_and_run_id, created_by_user, name, created, edited,
                 address, mail, phone, members, diets, needs_check)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
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
        .bind(data.needs_check)
        .execute(&mut *tx)
        .await;

        match result {
            Ok(_) => {
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

    #[tracing::instrument(skip(self))]
    pub async fn select_all_team(
        &self,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<Vec<(Team, Address)>, AppError> {
        let rows: Vec<TeamAddressRow> = sqlx::query_as(
            "SELECT
                t.id, t.cook_and_run_id, t.created_by_user, t.name, t.created, t.edited,
                t.address, t.mail, t.phone, t.members, t.diets, t.needs_check,
                a.id AS a_id, a.address_text AS a_address_text,
                a.latitude AS a_latitude, a.longitude AS a_longitude
             FROM team t
             INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
             INNER JOIN address a ON a.id = t.address
             WHERE car.id = $1 AND car.user_id = $2
             ORDER BY t.created ASC",
        )
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
        let row: TeamAddressRow = sqlx::query_as(
            "SELECT
                t.id, t.cook_and_run_id, t.created_by_user, t.name, t.created, t.edited,
                t.address, t.mail, t.phone, t.members, t.diets, t.needs_check,
                a.id AS a_id, a.address_text AS a_address_text,
                a.latitude AS a_latitude, a.longitude AS a_longitude
             FROM team t
             INNER JOIN cook_and_run car ON car.id = t.cook_and_run_id
             INNER JOIN address a ON a.id = t.address
             WHERE t.id = $1 AND car.id = $2 AND car.user_id = $3",
        )
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

    #[tracing::instrument(skip(self))]
    pub async fn delete_team(
        &self,
        id_filter: &Uuid,
        cook_and_run_id_filter: &Uuid,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
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
        .execute(&self.pool)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            return Err(AppError::TeamNotFound(
                *id_filter,
                user_id_filter.to_string(),
                *cook_and_run_id_filter,
            ));
        }
        Ok(())
    }

    #[tracing::instrument(skip(self, data, address_data))]
    pub async fn update_team(
        &self,
        data: &Team,
        address_data: &Address,
        user_id_filter: &str,
    ) -> Result<(), AppError> {
        let mut tx = self.pool.begin().await.map_err(AppError::DatabaseError)?;

        create_address(&mut *tx, address_data).await?;

        let affected = sqlx::query(
            "UPDATE team
             SET name = $1, edited = $2, address = $3, mail = $4,
                 phone = $5, members = $6, diets = $7, needs_check = $8
             WHERE id = $9
               AND (
                   cook_and_run_id IN (
                       SELECT id FROM cook_and_run WHERE id = $10 AND user_id = $11
                   )
                   OR created_by_user = $11
               )",
        )
        .bind(&data.name)
        .bind(data.edited)
        .bind(data.address)
        .bind(&data.mail)
        .bind(&data.phone)
        .bind(data.members)
        .bind(&data.diets)
        .bind(data.needs_check)
        .bind(data.id)
        .bind(data.cook_and_run_id)
        .bind(user_id_filter)
        .execute(&mut *tx)
        .await
        .map_err(AppError::DatabaseError)?
        .rows_affected();

        if affected == 0 {
            tx.rollback().await.map_err(AppError::DatabaseError)?;
            return Err(AppError::TeamNotFound(
                data.id,
                user_id_filter.to_string(),
                data.cook_and_run_id,
            ));
        }

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
    needs_check: bool,
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
                needs_check: row.needs_check,
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