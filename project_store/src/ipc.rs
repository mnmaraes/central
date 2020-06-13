macros::ipc! {
    Project {
        command => [
            Create { note_id: String } -> {
                create_project(&self.connection, &note_id)
            },
            Delete { id: String } -> {
                delete_project(&self.connection, &id)
            }
        ],
        query => [
            Get -> {
                get_all(&self.connection)
            } into Projects { projects: Vec<Project> = result.unwrap() } => {
                projects
            } as Vec<Project>
        ]
    }
}
