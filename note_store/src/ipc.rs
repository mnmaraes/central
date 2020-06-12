use macros::ipc;

use super::models::{create_note, delete_note, get_all, update_note, Note};

ipc! {
    Note {
        command => [
            Create { body: String } -> {
                create_note(&self.connection, &body)
            },
            Update { id: String, body: String } -> {
                update_note(&self.connection, &id, &body)
            },
            Delete { id: String } -> {
                delete_note(&self.connection, &id)
            }
        ],
        query => [
            Get -> {
                get_all(&self.connection)
            } into Notes { notes: Vec<Note> = result.unwrap() } => {
                notes
            } as Vec<Note>
        ]
    }
}
