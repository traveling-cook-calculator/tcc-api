use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Course {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub time: String,
    pub has_multiple_hosts: bool,
}
