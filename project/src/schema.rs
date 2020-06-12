table! {
    note (id) {
        id -> Uuid,
        body -> Varchar,
        createdAt -> Timestamp,
        updatedAt -> Timestamp,
    }
}

table! {
    notes (id) {
        id -> Uuid,
        body -> Text,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    projects (id) {
        id -> Uuid,
        key_note -> Uuid,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

joinable!(projects -> notes (key_note));

allow_tables_to_appear_in_same_query!(
    note,
    notes,
    projects,
);
