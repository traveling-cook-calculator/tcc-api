// The API layer addresses this type as `crate::point::Point` (see
// `PointDTO::from_domain`/`to_domain`), separately from `crate::project`,
// even though it's defined alongside `Project` in `domain::project`. This
// module exists purely to satisfy that path; all point persistence goes
// through `project::set_project_start_point` and friends, which call
// `infrastructure::db::point::PointRepository` directly.
pub use crate::domain::project::Point;
