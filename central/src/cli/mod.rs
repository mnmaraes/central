mod notes;

use clap::Clap;

use notes::Note;

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
    Note(Note),
}

impl Commands {
    fn run(&self) {
        use Commands::*;

        match self {
            Note(note) => note.run(),
        };
    }
}
