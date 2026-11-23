
use std::{fs::File, io::BufReader, path::Path, sync::Arc};

use rodio::{Decoder, DeviceSinkBuilder, DeviceSinkError, MixerDeviceSink, decoder::DecoderError, queue::{SourcesQueueInput, queue}};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Decoder Error: {0}")]
    DecoderError(#[from] DecoderError),
}

pub struct Player {
    sink: MixerDeviceSink,
    player: rodio::Player,
    queue: Arc<SourcesQueueInput>,
}

impl Player {
    pub fn new() -> Result<Self, DeviceSinkError> {
        let sink = DeviceSinkBuilder::open_default_sink()?;
        let (input, output) = queue(true);
        let player = Self {
            player: rodio::Player::connect_new(&sink.mixer()),
            sink,
            queue: input,
        };
        
        player.player.append(output);
        
        Ok(player)
    }
    
    pub fn play(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        
        let source = Decoder::builder()
            .with_byte_len(metadata.len())
            .with_seekable(true)
            .with_hint(path.extension().map(|s| s.to_str()).flatten().unwrap_or(""))
            .with_data(BufReader::new(file))
            .build()?;
        
        self.queue.append(source);
        Ok(())
    }
    
    pub fn clear(&self) { self.queue.clear(); }
}
