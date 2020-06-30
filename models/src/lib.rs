#[cfg(feature = "store")]
#[macro_use]
extern crate diesel;

#[cfg(feature = "store")]
mod schema;

#[cfg(feature = "store")]
pub use schema::*;

#[cfg(feature = "notes")]
use std::time::SystemTime;

#[cfg(feature = "notes")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "notes")]
use uuid::Uuid;

#[cfg(feature = "notes")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "store", derive(Queryable, Identifiable))]
pub struct Note {
    pub id: Uuid,
    pub body: String,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}

#[cfg(feature = "notes")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum NoteRef {
    Deferred,
    Path(String),
    Id(String),
}

#[cfg(feature = "notes")]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteDescriptor {
    pub reference: NoteRef,
    pub title: String,
}

impl From<Note> for NoteDescriptor {
    fn from(note: Note) -> Self {
        let title = note
            .body
            .lines()
            .next()
            .unwrap_or("")
            .replace("#", "")
            .trim()
            .to_string();

        NoteDescriptor {
            reference: NoteRef::Id(note.id.to_string()),
            title,
        }
    }
}

#[cfg(feature = "projects")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "store", derive(Queryable, Identifiable))]
pub struct Project {
    pub id: Uuid,
    pub key_note: Uuid,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
}
