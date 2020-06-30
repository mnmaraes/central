macros::ipc! {
    Note {
        command => [
            Create { reference: NoteRef, body: String } -> {
                match reference {
                    NoteRef::Deferred => create_note(&self.connection, &body),
                    _ => Err(format_err!("NoteStore::Create requires a Deferred note reference"))
                }
            },
            Update { reference: NoteRef, body: String } -> {
                match reference {
                    NoteRef::Id(id) => update_note(&self.connection, &id, &body),
                    _ => Err(format_err!("NoteStore::Update requires an Id note reference"))
                }
            },
            Delete { reference: NoteRef } -> {
                match reference {
                    NoteRef::Id(id) => delete_note(&self.connection, &id),
                    _ => Err(format_err!("NoteStore::Update requires an Id note reference"))
                }
            }
        ],
        query => [
            GetContent { reference: NoteRef } -> {
                if let NoteRef::Id(id) = reference {
                    get_note(&self.connection, &id)
                } else {
                    Err(format_err!("Note not found"))
                }
            } into Content { content: String = result.unwrap().body } => {
                content
            } as String,
            GetIndex -> {
                get_all_descriptors(&self.connection)
            } into Index { index: Vec<NoteDescriptor> = result.unwrap() } => {
                index
            } as Vec<NoteDescriptor>
        ]
    }
}
