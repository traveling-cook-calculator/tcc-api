use uuid::Uuid;

use crate::{auth::get_auth0_1, course::post_test::create_course, create_cook_and_run};

mod delete_test;
mod get_test;
mod post_test;

mod patch_test;

pub fn setup() -> (Uuid, Uuid) {
    let cook_and_run_id = create_cook_and_run();

    let (token, _) = get_auth0_1();
    let course_id = Uuid::new_v4();
    create_course(&cook_and_run_id, &course_id, &token);

    (cook_and_run_id, course_id)
}
