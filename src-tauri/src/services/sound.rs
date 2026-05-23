#![allow(clippy::module_name_repetitions)]

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use rodio::source::{SineWave, Source, Zero};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::error::{AppError, Result};
use crate::services::timer::SoundType;
use crate::services::Service;
use crate::services::ServiceContext;

/// Commands consumed by the dedicated audio thread.
#[derive(Debug)]
pub(crate) enum SoundCommand {
    Play(SoundAsset),
    #[allow(dead_code)]
    Stop,
    #[allow(dead_code)]
    SetEnabled(bool),
    Shutdown,
}

/// Audio assets currently supported by the service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SoundAsset {
    PreAlert,
    RestComplete,
}

impl From<SoundType> for SoundAsset {
    fn from(value: SoundType) -> Self {
        match value {
            SoundType::PreAlert => Self::PreAlert,
            SoundType::RestComplete => Self::RestComplete,
        }
    }
}

pub(crate) struct SoundService {
    tx: mpsc::Sender<SoundCommand>,
    thread_handle: Mutex<Option<thread::JoinHandle<()>>>,
}

impl SoundService {
    /// Spawn the audio thread and return the service handle.
    ///
    /// # Errors
    ///
    /// Returns an error when the audio thread cannot be spawned.
    pub(crate) fn new() -> Result<Self> {
        let (tx, rx) = mpsc::channel(32);
        let thread_handle = thread::Builder::new()
            .name("eyezen-audio".to_string())
            .spawn(move || Self::audio_thread_loop(rx))
            .map_err(|err| AppError::IoError {
                message: format!("failed to spawn audio thread: {err}"),
            })?;

        info!("sound service audio thread spawned");
        Ok(Self {
            tx,
            thread_handle: Mutex::new(Some(thread_handle)),
        })
    }

    /// Queue playback of a sound asset.
    pub(crate) fn play(&self, asset: SoundAsset) {
        if let Err(err) = self.tx.try_send(SoundCommand::Play(asset)) {
            warn!("failed to send play command: {err}");
        }
    }

    /// Queue playback by timer sound type.
    pub(crate) fn play_type(&self, sound: SoundType) {
        self.play(sound.into());
    }

    #[allow(dead_code)]
    pub(crate) fn stop(&self) {
        if let Err(err) = self.tx.try_send(SoundCommand::Stop) {
            warn!("failed to send stop command: {err}");
        }
    }

    #[allow(dead_code)]
    pub(crate) fn set_enabled(&self, enabled: bool) {
        if let Err(err) = self.tx.try_send(SoundCommand::SetEnabled(enabled)) {
            warn!("failed to send enabled command: {err}");
        }
    }

    fn audio_thread_loop(mut rx: mpsc::Receiver<SoundCommand>) {
        let output = match OutputStream::try_default() {
            Ok(output) => Some(output),
            Err(err) => {
                warn!("no audio output available: {err}");
                None
            }
        };

        let mut enabled = true;
        let mut current_sink: Option<Sink> = None;

        while let Some(command) = rx.blocking_recv() {
            match command {
                SoundCommand::Play(asset) => {
                    if !enabled {
                        continue;
                    }

                    if let Some((_, handle)) = output.as_ref() {
                        match Self::play_asset(handle, asset) {
                            Ok(sink) => {
                                current_sink = Some(sink);
                            }
                            Err(err) => {
                                warn!("failed to play sound asset {asset:?}: {err}");
                            }
                        }
                    }
                }
                SoundCommand::Stop => {
                    if let Some(sink) = current_sink.take() {
                        sink.stop();
                    }
                }
                SoundCommand::SetEnabled(value) => {
                    enabled = value;
                    if !enabled {
                        if let Some(sink) = current_sink.take() {
                            sink.stop();
                        }
                    }
                    info!("sound enabled set to {value}");
                }
                SoundCommand::Shutdown => {
                    if let Some(sink) = current_sink.take() {
                        sink.stop();
                    }
                    info!("sound service audio thread shutting down");
                    break;
                }
            }
        }
    }

    fn play_asset(
        handle: &OutputStreamHandle,
        asset: SoundAsset,
    ) -> std::result::Result<Sink, AppError> {
        let sink = Sink::try_new(handle).map_err(|err| AppError::IoError {
            message: format!("failed to create sink: {err}"),
        })?;

        match asset {
            SoundAsset::PreAlert => {
                sink.append(Self::beep(880.0, 160));
            }
            SoundAsset::RestComplete => {
                sink.append(Self::beep(660.0, 140));
                sink.append(Self::silence(80));
                sink.append(Self::beep(880.0, 160));
            }
        }

        Ok(sink)
    }

    fn beep(freq: f32, millis: u64) -> impl Source<Item = f32> + Send {
        SineWave::new(freq)
            .take_duration(Duration::from_millis(millis))
            .amplify(0.15)
    }

    fn silence(millis: u64) -> impl Source<Item = f32> + Send {
        Zero::new(1, 48_000).take_duration(Duration::from_millis(millis))
    }
}

impl Service for SoundService {
    async fn init(&self, _app: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn start(&self, _app: &ServiceContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        if let Err(err) = self.tx.send(SoundCommand::Shutdown).await {
            warn!("failed to send sound shutdown command: {err}");
        }
        Ok(())
    }
}

impl Drop for SoundService {
    fn drop(&mut self) {
        // Drop must not panic and cannot return errors. try_send fails only
        // if the worker thread has already exited (channel closed) or its
        // queue is saturated; in both cases the thread will terminate on its
        // own when the Sender is dropped below.
        let _ = self.tx.try_send(SoundCommand::Shutdown);

        if let Ok(mut handle) = self.thread_handle.lock() {
            if let Some(join_handle) = handle.take() {
                // join returns Err only if the worker thread panicked. We
                // are already in Drop so there is nothing actionable; the
                // panic was logged at the source.
                let _ = join_handle.join();
            }
        }
    }
}

// Skipped on Windows: rodio's OutputStream init occasionally faults with
// STATUS_ACCESS_VIOLATION on headless GitHub Actions runners that have no
// audio device. Linux/macOS runners cover the same code paths.
#[cfg(all(test, not(target_os = "windows")))]
mod tests {
    use super::*;

    #[test]
    fn new_creates_service() {
        let service = SoundService::new();
        assert!(service.is_ok(), "sound service should create");
    }

    #[test]
    fn play_command_does_not_panic() {
        let service = SoundService::new().expect("sound service should initialize");
        service.play(SoundAsset::PreAlert);
    }

    #[test]
    fn set_enabled_false_suppresses_play() {
        let service = SoundService::new().expect("sound service should initialize");
        service.set_enabled(false);
        service.play(SoundAsset::PreAlert);
    }
}
