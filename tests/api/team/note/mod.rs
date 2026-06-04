use uuid::Uuid;

use crate::{
    auth::get_user_1,
    team::{self, note::post_test::create_note},
};

mod delete_test;
mod get_test;
mod post_test;

pub fn setup() -> (Uuid, Uuid, Uuid) {
    let (cook_and_run_id, team_id) = team::setup();

    let (token, _) = get_user_1();
    let note_id = Uuid::new_v4();
    create_note(&cook_and_run_id, &team_id, &note_id, &token);

    (cook_and_run_id, team_id, note_id)
}
