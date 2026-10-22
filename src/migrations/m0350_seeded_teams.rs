//! Allow seeded empty teams by making `Teams.captain_id` nullable.
//!
//! Seeded teams start with `captain_id = NULL` and an authoritative invite code.
//! The first member who joins via `POST /api/team/accept` is automatically
//! assigned as the team captain.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

pub(crate) const UP_SQL: &str = r#"
ALTER TABLE "Teams" ALTER COLUMN captain_id DROP NOT NULL;
"#;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::UP_SQL;

    #[test]
    fn seeded_teams_migration_drops_not_null_on_captain_id() {
        assert!(UP_SQL.contains(r#"ALTER TABLE "Teams" ALTER COLUMN captain_id DROP NOT NULL;"#));
    }
}
