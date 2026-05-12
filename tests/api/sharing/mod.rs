mod delete_test;
mod get_test;
mod patch_test;
mod post_team_test;
mod post_test;
use chrono::NaiveDateTime;
use uuid::Uuid;

use crate::{auth::get_auth0_1, create_cook_and_run, sharing::post_test::create_share_config};

pub fn setup() -> Uuid {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();
    create_share_config(
        &cook_and_run_id,
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
        &Some(NaiveDateTime::parse_from_str("2015-09-05 23:56", "%Y-%m-%d %H:%M").unwrap()),
    );
    cook_and_run_id
}

pub fn get_share_config(cook_and_run_id: &Uuid, token: &str) {
    get_test::get_share_config(
        cook_and_run_id,
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
        &Some("2015-09-05T23:56"),
    );
}
