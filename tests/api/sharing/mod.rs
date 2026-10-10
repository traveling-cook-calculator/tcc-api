mod delete_test;
mod get_test;
mod patch_test;
pub mod post_team_test;
pub mod post_test;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{auth::get_user_1, create_project, sharing::post_test::create_share_config_default};

pub fn setup() -> Uuid {
    let project_id = create_project();

    let (token, _) = get_user_1();
    create_share_config_default(
        &project_id,
        &token,
        true,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &Some(5),
        &Some(
            DateTime::parse_from_rfc3339("2015-09-05T23:56:00Z")
                .unwrap()
                .with_timezone(&Utc),
        ),
    );
    project_id
}

pub fn get_share_config(project_id: &Uuid, token: &str) {
    get_test::get_share_config(
        project_id,
        token,
        true,
        true,
        &vec![
            "mail".to_string(),
            "phone".to_string(),
            "members".to_string(),
            "diets".to_string(),
        ],
        &Some(5),
        &Some("2015-09-05T23:56:00Z"),
        &None,
        &vec![],
        false,
        false,
        false,
    );
}
