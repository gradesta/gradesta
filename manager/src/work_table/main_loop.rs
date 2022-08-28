use super::configuration::Configuration;
use anyhow;

pub async fn run(config: &Configuration) -> anyhow::Result<()> {
    // launch the manager
    //  Watch socket dir for new clients
    //  Listen on websocket for new clients
    //  Respond to requests and keep everything in sync
    Ok(())
}
