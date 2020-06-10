mod notes;
mod status;

use clap::Clap;

use notes::Note;
use status::Status;

/// The cli for Central
#[derive(Clap)]
#[clap(version = "0.1.0", author = "Murillo N. Maraes <mnmaraes@gmail.com>")]
pub struct Central {
    #[clap(subcommand)]
    subcmd: Commands,
}

impl Central {
    pub fn run(&self) {
        self.subcmd.run();
    }
}

#[derive(Clap)]
enum Commands {
    Status(Status),
    Note(Note),
}

impl Commands {
    fn run(&self) {
        use Commands::*;

        match self {
            Status(status) => status.run(),
            Note(note) => note.run(),
        };
    }
}
