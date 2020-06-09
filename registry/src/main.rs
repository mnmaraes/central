mod lib;

use actix::prelude::*;

use failure::Error;

use crate::lib::Registry;

use tracing::{error, info, span, Level};

#[actix_rt::main]
async fn main() -> Result<(), Error> {
    dotenv::dotenv()?;
    let log_dir = std::env::var("LOG_DIRECTORY")?;

    let file_appender = tracing_appender::rolling::daily(&log_dir, "registry.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    let result: Result<_, Error> = {
        let span = span!(Level::TRACE, "Registry Running");
        let _enter = span.enter();

        Registry::serve_default()?;

        tokio::signal::ctrl_c().await?;
        info!("Ctrl-C received, shutting down");

        Ok(())
    };

    if let Err(e) = result {
        error!("Error: {:?} caused program to exit", e);
    }

    System::current().stop();

    Ok(())
}
