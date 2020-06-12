mod runners;

use clap::Clap;

use cli::cli;

use runners::*;

cli! {
    /// The cli for Central
    Central.[
        /// Central services status checks
        Status => check_status,
        /// Manages the raw note data. Use with care
        Note.[
            /// Creates a new note and saves it to central
            New => create_note,
            /// Selects and deletes an existing note
            Delete => delete_note,
            /// Selects and updates an existing note
            Update => update_note
        ]
    ]
}
