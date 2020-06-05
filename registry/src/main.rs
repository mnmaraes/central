mod lib;

use actix::prelude::*;

use failure::Error;

use crate::lib::Registry;

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    Registry::serve_default()?;

    tokio::signal::ctrl_c().await?;
    println!("Ctrl-C received, shutting down");

    System::current().stop();

    Ok(())
}
