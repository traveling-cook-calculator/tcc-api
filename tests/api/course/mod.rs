use uuid::Uuid;

use crate::{auth::get_user_1, course::post_test::create_course, create_project};

mod delete_test;
mod get_test;
mod post_test;

mod patch_test;

pub fn setup() -> (Uuid, Uuid) {
    let project_id = create_project();

    let (token, _) = get_user_1();
    let course_id = Uuid::new_v4();
    create_course(&project_id, &course_id, &token);

    (project_id, course_id)
}
