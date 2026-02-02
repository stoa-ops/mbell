use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use thiserror::Error;
use tracing::{debug, error, info};

// Embed the bowl sound at compile time
const BOWL_SOUND: &[u8] = include_bytes!("../assets/bowl.ogg");

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to initialize audio output: {0}")]
    OutputError(String),
    #[error("Failed to decode audio: {0}")]
    DecodeError(String),
    #[error("Playback error: {0}")]
    PlaybackError(String),
}

pub struct AudioPlayer {
    volume: f32,
}

impl AudioPlayer {
    pub fn new(volume: u8) -> Self {
        Self {
            volume: volume as f32 / 100.0,
        }
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume as f32 / 100.0;
    }

    pub fn play(&self) -> Result<(), AudioError> {
        debug!("Playing bell sound at volume {:.0}%", self.volume * 100.0);

        // Get output stream - rodio auto-detects backend (PipeWire -> PulseAudio -> ALSA)
        let (_stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| AudioError::OutputError(e.to_string()))?;

        let sink = Sink::try_new(&stream_handle)
            .map_err(|e| AudioError::PlaybackError(e.to_string()))?;

        // Decode the embedded OGG file
        let cursor = Cursor::new(BOWL_SOUND);
        let source = Decoder::new(cursor)
            .map_err(|e| AudioError::DecodeError(e.to_string()))?;

        sink.set_volume(self.volume);
        sink.append(source);
        sink.sleep_until_end();

        info!("Bell played successfully");
        Ok(())
    }

    pub fn play_async(&self) {
        let volume = self.volume;
        tokio::task::spawn_blocking(move || {
            if let Err(e) = play_with_volume(volume) {
                error!("Failed to play bell: {}", e);
            }
        });
    }
}

fn play_with_volume(volume: f32) -> Result<(), AudioError> {
    let (_stream, stream_handle) = OutputStream::try_default()
        .map_err(|e| AudioError::OutputError(e.to_string()))?;

    let sink = Sink::try_new(&stream_handle)
        .map_err(|e| AudioError::PlaybackError(e.to_string()))?;

    let cursor = Cursor::new(BOWL_SOUND);
    let source = Decoder::new(cursor)
        .map_err(|e| AudioError::DecodeError(e.to_string()))?;

    sink.set_volume(volume);
    sink.append(source);
    sink.sleep_until_end();

    Ok(())
}

/// Ring the bell once (convenience function)
pub fn ring(volume: u8) -> Result<(), AudioError> {
    let player = AudioPlayer::new(volume);
    player.play()
}

/// Ring the bell asynchronously (non-blocking)
pub fn ring_async(volume: u8) {
    let player = AudioPlayer::new(volume);
    player.play_async();
}
