mod cli;

use clap::Clap;

use cli::Central;

fn main() {
    let central = Central::parse();
    central.run();
}
