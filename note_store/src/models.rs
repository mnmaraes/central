use std::time::SystemTime;

use failure::Error;

use diesel::prelude::*;

use uuid::Uuid;

use super::schema::notes;

#[derive(Queryable, Identifiable)]
pub struct Note {
    pub id: Uuid,
    pub body: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[derive(Insertable)]
#[table_name = "notes"]
pub struct NewNote<'a> {
    pub body: &'a str,
}

pub fn create_note(conn: &PgConnection, body: &str) -> Result<(), Error> {
    let new_note = NewNote { body };

    diesel::insert_into(notes::table)
        .values(&new_note)
        .execute(conn)?;

    Ok(())
}

pub fn update_note(conn: &PgConnection, note_id: &str, new_body: &str) -> Result<(), Error> {
    use self::notes::dsl::*;

    let note_id = Uuid::parse_str(note_id)?;
    diesel::update(notes.find(note_id))
        .set(body.eq(new_body))
        .execute(conn)?;

    Ok(())
}

pub fn delete_note(conn: &PgConnection, note_id: &str) -> Result<(), Error> {
    use self::notes::dsl::*;

    let note_id = Uuid::parse_str(note_id)?;
    diesel::delete(notes.find(note_id)).execute(conn)?;

    Ok(())
}
