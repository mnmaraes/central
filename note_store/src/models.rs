use failure::Error;

use diesel::prelude::*;

use uuid::Uuid;

use models::notes;
pub use models::Note;

#[derive(Insertable)]
#[table_name = "notes"]
struct NewNote<'a> {
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

pub fn get_all(conn: &PgConnection) -> Result<Vec<Note>, Error> {
    use self::notes::dsl::*;

    let results = notes.load::<Note>(conn)?;

    Ok(results)
}
