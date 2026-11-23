
use fipc::server::*;
use fmp_interface::SERVER_NAME;
use tokio::sync::mpsc;

use utils::*;

mod utils;
mod client;
mod player;
mod server;

const COMMAND_LIMIT: usize = 256;
const MESSAGE_LIMIT: usize = 16;

#[tokio::main]
async fn main() {
    #[cfg(feature = "env_logger")]
    env_logger::init();
    
    let name = ServerName::new(SERVER_NAME).unwrap();
    let (command_sender, command_receiver) = mpsc::channel(COMMAND_LIMIT);
    let (request_sender, request_receiver) = mpsc::channel(1);
    
    let Some(server) = Server::listen(&name, move |client| {
        tokio::spawn(client::new_client(client, request_sender.clone(), command_sender.clone()));
    }).log() else { return };
    
    server::server(server, request_receiver, command_receiver).await
}
