#[macro_use]
extern crate diesel;

mod models;
mod schema;

use failure::Error;

use actix::prelude::*;

use cliff::{client, router};

pub struct NoteStore;

impl Actor for NoteStore {
    type Context = Context<Self>;
}

router! {
    NoteStore;
    [
        NoteCommand [
            Create { body: String } -> {
                //TODO: Store our note
            } => Success,
            Update { id: String } -> {
                //TODO: Update our note
            } => Success,
            Delete { id: String } -> {
                //TODO: Delete our note
            } => Success
        ],
        NoteQuery [
            Get { id: String } -> {
                // TODO: Retrieve the note
            } => NotFound
        ]
    ]
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    println!("Hello, world!");

    tokio::signal::ctrl_c().await?;
    System::current().stop();

    Ok(())
}
