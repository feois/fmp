
use crate::utils::*;

use std::{str::FromStr, sync::Arc};

use fmp_interface::{client_name::ClientNameError, command::{ClientCommand, Command, ServerCommand}, message::Message};
use thiserror::Error;
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter}, select, sync::{mpsc, oneshot}, task::JoinError};

use fipc::server::*;

pub struct Request {
    pub name: Arc<str>,
    pub response: oneshot::Sender<Result<Terminate, ClientNameError>>,
}

pub type Terminate = oneshot::Receiver<()>;

struct Stream<T>(tokio::io::Lines<tokio::io::BufReader<T>>);

#[derive(Error, Debug)]
enum Error {
    #[error("Client terminated communication unexpectedly")]
    Terminated,
    #[error("Server unresponsive, shutting down")]
    Shutdown,
    #[error(transparent)]
    Io(#[from] IOError),
    #[error("Task panic: {0}")]
    Panic(#[from] JoinError),
    #[error("Cannot send message to task")]
    MessagerFail,
}

impl From<io::Error> for Error { fn from(value: io::Error) -> Self { IOError(value).into() } }
impl From<oneshot::error::RecvError> for Error { fn from(_: oneshot::error::RecvError) -> Self { Error::Shutdown } }

impl<T: io::AsyncRead + Unpin> Stream<T> {
    fn new(io: T) -> Self { Self(BufReader::new(io).lines()) }
    
    async fn read(&mut self, name: &ClientName) -> Result<Result<Command, <Command as FromStr>::Err>, Error> {
        let Self(io) = self;
        let Some(s) = io.next_line().await? else { return Err(Error::Terminated) };
        log::trace!(target: "command", "{} Command: {}", name, &s[..s.len() - 1]);
        Ok(Command::from_str(&s))
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct ClientName(Option<Arc<str>>);

impl ClientName {
    pub const fn name(name: Arc<str>) -> Self { Self(Some(name)) }
    pub const fn unidentified() -> Self { Self(None) }
    pub fn log<E: std::error::Error>(&self, e: E) { log::error!("{} {}", self, e) }
    pub fn as_str(&self) -> Option<&str> { let Self(name) = self; name.as_ref().map(|name| name.as_ref()) }
}

impl std::fmt::Display for ClientName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self(Some(name)) = self { write!(f, "[{}]", name) }
        else { write!(f, "<Unidentified>") }
    }
}

async fn messager(mut identify: oneshot::Receiver<ClientName>, mut stream: mpsc::Receiver<Message>, writer: impl io::AsyncWrite + Unpin) -> io::Result<()> {
    let mut name = ClientName::unidentified();
    let mut writer = BufWriter::new(writer);
    
    while let Some(message) = stream.recv().await {
        let message = message.to_line();
        
        log::trace!(target: "message", "{} Message: {}", name, &message[..message.len() - 1]);
        
        writer.write_all(message.as_bytes()).await?;
        
        if stream.is_empty() { writer.flush().await? }
        if let Ok(n) = identify.try_recv() { name = n }
    }
    
    writer.flush().await
}

async fn identify(stream: &mut Stream<impl io::AsyncRead + Unpin>, request: mpsc::Sender<Request>, message: &mpsc::Sender<Message>) -> Result<(Arc<str>, Terminate), Error> {
    let name = ClientName::unidentified();
    
    log::info!("New client connection requested");
    
    loop {
        if let Ok(Command::Client(ClientCommand::Identify { name })) = stream.read(&name).await? {
            let name: Arc<str> = name.into();
            let (sender, receiver) = oneshot::channel();
            request.send(Request { name: name.clone(), response: sender }).await.map_err(|_| Error::Shutdown)?;
            
            match receiver.await? {
                Ok(t) => {
                    message.send(Message::Identified).await.map_err(|_| Error::MessagerFail)?;
                    return Ok((name, t));
                }
                Err(e) => {
                    log::warn!("{} {}", ClientName::unidentified(), e);
                    message.send(Message::IdentificationError(e)).await.map_err(|_| Error::MessagerFail)?;
                }
            }
        }
    }
}

pub async fn new_client(client: ConnectedClient, request: mpsc::Sender<Request>, command_sender: mpsc::Sender<ServerCommand>) {
    let (reader, writer) = io::split(client);
    let mut reader = Stream::new(reader);
    
    let (message_sender, message_receiver) = mpsc::channel(crate::MESSAGE_LIMIT);
    let (identify_sender, identify_receiver) = oneshot::channel();
    let mut messager = tokio::spawn(messager(identify_receiver, message_receiver, writer));
    
    let Ok((name, mut terminate)) = identify(&mut reader, request, &message_sender).await
        .inspect_err(|e| ClientName::unidentified().log(e)) else { return };
    let c = ClientName::name(name);
    if let Err(()) = identify_sender.send(c.clone()).map_err(|_| c.log(Error::MessagerFail)) { return }
    
    log::info!("{} Connected", c);
    
    loop {
        select!{
            command = reader.read(&c) => match command {
                Ok(Ok(Command::Client(command))) => {
                    
                }
                Ok(Ok(Command::Server(command))) => {
                    let t = matches!(&command, ServerCommand::Disconnect { client } if client == c.as_str().unwrap());
                    if let Err(_) = command_sender.send(command).await { break c.log(Error::Shutdown) }
                    if t { break }
                }
                Ok(Err(e)) => log::warn!("{} Invalid command: {}", c, e),
                Err(e) => break c.log(e),
            },
            write_result = &mut messager => {
                if let Err(e) = write_result { break c.log(e) }
            }
            _ = &mut terminate => break message_sender.try_send(Message::Terminated).ignore(),
        }
    }
    
    log::info!("{} Disconnecting", c);
    command_sender.send(ServerCommand::Disconnect { client: c.as_str().unwrap().to_owned() }).await.ignore();
    messager.abort();
}
