#[derive(Queryable)]
pub struct Note {
    pub id: uuid::Uuid,
    pub body: String,
}
