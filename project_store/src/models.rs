use failure::Error;

use diesel::prelude::*;

use uuid::Uuid;

use models::projects;

pub use models::Project;

#[derive(Insertable)]
#[table_name = "projects"]
pub struct NewProject {
    pub key_note: Uuid,
}

pub fn create_project(conn: &PgConnection, note_id: &str) -> Result<(), Error> {
    let new_project = NewProject {
        key_note: Uuid::parse_str(note_id)?,
    };

    diesel::insert_into(projects::table)
        .values(&new_project)
        .execute(conn)?;

    Ok(())
}

pub fn delete_project(conn: &PgConnection, project_id: &str) -> Result<(), Error> {
    use self::projects::dsl::*;

    let project_id = Uuid::parse_str(project_id)?;
    diesel::delete(projects.find(project_id)).execute(conn)?;

    Ok(())
}

pub fn get_all(conn: &PgConnection) -> Result<Vec<Project>, Error> {
    use self::projects::dsl::*;

    let results = projects.load::<Project>(conn)?;

    Ok(results)
}
