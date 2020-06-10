use std::env;

use failure::{format_err, Error};

use actix::prelude::*;

use cliff::{client, router};

use diesel::prelude::*;

use super::models::*;

pub struct NoteStore {
    connection: PgConnection,
}

impl Default for NoteStore {
    fn default() -> Self {
        dotenv::dotenv().expect("Unable to load environment");

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL env var not found");

        let connection = PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Couldn't connect to {}", database_url));

        NoteStore { connection }
    }
}

impl Actor for NoteStore {
    type Context = Context<Self>;
}

router! {
    NoteStore;
    [
        NoteCommand [
            Create { body: String } -> {
                let res = create_note(&self.connection, &body);
            } => [
                let Err(e) = res => Error [String] { description: format!("Error Creating Note: {}", e) },
                => Success
            ],
            Update { id: String, body: String } -> {
                let res = update_note(&self.connection, &id, &body);
            } => [
                let Err(e) = res => Error { description: format!("Error Creating Note: {}", e) },
                => Success
            ],
            Delete { id: String } -> {
                let res = delete_note(&self.connection, &id);
            } =>[
                let Err(e) = res => Error { description: format!("Error Creating Note: {}", e) },
                => Success
            ]
        ],
        NoteQuery [
            Get -> {
                let res = get_all(&self.connection);
            } => [
                let Err(e) = res => Error [String] { description: format!("Error Fetching Notes: {}", e) },
                => Notes [Vec<Note>] { notes: res.unwrap() }
            ]
        ],
        NoteStoreStatus [
            Check => Alive
        ]
    ]
}

client! {
    NoteCommand {
        actions => [
            Create { body: String } wait,
            Update { id: String, body: String } wait,
            Delete { id: String } wait
        ],
        response_mapping => [
            Success => [ () ],
            Error { description: _ } => [ () ]
        ]
    }
}

client! {
    NoteQuery {
        actions => [
            Get wait Result<Vec<Note>, Error>,
        ],
        response_mapping => [
            Notes { notes } => [
                Result<Vec<Note>, Error>: Ok(notes)
            ],
            Error { description  } => [
                Result<Vec<Note>, Error>: Err(format_err!("{}",description))
            ]
        ]
    }
}

client! {
    NoteStoreStatus {
        actions => [
            Check wait,
        ],
        response_mapping => [
            Alive => [ () ]
        ]
    }
}
