macros::ipc! {
    Project {
        command => [
            Create { name: String } -> {
                create_project(&self.connection, &name)
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
