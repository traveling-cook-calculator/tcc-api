mod delete_location_test;
mod delete_test;
pub mod get_location_test;
pub mod get_test;
mod patch_location_test;
mod patch_meta_test;
pub mod post_test;

/// Default `admin_notification_email` used by test fixtures that don't
/// exercise the value itself. Became mandatory on project creation
/// (`POST /project/{id}`) and metadata updates
/// (`PATCH /project/{id}/metadata`) in v0.2.0.
pub const DEFAULT_ADMIN_NOTIFICATION_EMAIL: &str = "admin-notifications@cook-and-run.test";
