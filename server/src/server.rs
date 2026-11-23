
use std::collections::{HashMap, HashSet};

use fipc::server::*;
use fmp_interface::{client_name::{self, ClientNameError}, command::ServerCommand};
use tokio::{select, sync::{mpsc, oneshot}};

use crate::{player::Player, utils::*};
use crate::client::Request;

struct State {
    clients: HashSet<String>,
}

pub async fn server(mut server: Server, mut requests: mpsc::Receiver<Request>, mut commands: mpsc::Receiver<ServerCommand>) {
    let Ok(mut player) = Player::new().inspect_err(|e| log::error!("Failed to create player: {}", e)) else { return };
    let mut clients = HashMap::new();
    
    log::info!("Server started successfully");
    
    loop {
        select!{
            error = server.wait() => break error.log_discard(),
            
            request = requests.recv() => {
                let Request { name, response } = request.unwrap();
                
                if let Some(e) = client_name::validate(&name)
                    .or_else(|| clients.contains_key(&name).then_some(ClientNameError::Clashed))
                { response.send(Err(e)).ignore() }
                else {
                    let (term_send, term_recv) = oneshot::channel();
                    
                    if response.send(Ok(term_recv)).is_ok() { clients.insert(name, term_send); }
                    else { log::error!("Client {} terminated unexpectedly", &name) }
                }
            }
            
            command = commands.recv() => {
                use ServerCommand::*;
                
                match command.unwrap() {
                    Disconnect { client } => if let Some(terminate) = clients.remove(client.as_str()) {
                        terminate.send(()).ignore();
                        log::info!("Disconnected client {}", &client);
                    },
                    Play { path } => player.play(path).log_discard(),
                    Clear => player.clear(),
                    Exit => break,
                }
            }
        }
    }
    
    for terminate in clients.into_values() { terminate.send(()).ignore() }
    
    log::info!("Exiting");
}
