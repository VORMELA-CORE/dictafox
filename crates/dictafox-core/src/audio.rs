use std::path::Path;
#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

pub const SAMPLE_RATE: u32 = 16_000;
const BYTES_PER_SAMPLE: usize = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_monitor: bool,
    pub is_default: bool,
}

pub struct RecordedAudio {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub peak: f32,
}

impl RecordedAudio {
    pub fn duration_ms(&self) -> u64 {
        (self.samples.len() as u64 * 1000) / self.sample_rate as u64
    }
}

pub fn command_exists(name: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| {
        dir.join(name).is_file() || (cfg!(windows) && dir.join(format!("{name}.exe")).is_file())
    })
}

/// Samples captured so far plus live meters, shared between the capture
/// thread (or audio callback) and the UI-facing `Recorder`.
struct Sink {
    samples: Mutex<Vec<i16>>,
    level: Mutex<f32>,
    peak: Mutex<f32>,
    finished: AtomicBool,
    captured: AtomicU32,
    max_samples: usize,
}

impl Sink {
    fn new(max_samples: usize, sample_rate: u32) -> Self {
        Self {
            samples: Mutex::new(Vec::with_capacity(max_samples.min(sample_rate as usize * 30))),
            level: Mutex::new(0.0),
            peak: Mutex::new(0.0),
            finished: AtomicBool::new(false),
            captured: AtomicU32::new(0),
            max_samples,
        }
    }

    /// Append mono PCM and update meters. Returns false once the maximum
    /// recording length is reached.
    fn ingest(&self, frame: &[i16]) -> bool {
        if frame.is_empty() {
            return true;
        }
        let mut frame_peak: i16 = 0;
        let mut sum_squares: u64 = 0;
        {
            let Ok(mut guard) = self.samples.lock() else {
                return false;
            };
            for sample in frame {
                frame_peak = frame_peak.max(sample.saturating_abs());
                sum_squares += (*sample as i64 * *sample as i64) as u64;
                if guard.len() < self.max_samples {
                    guard.push(*sample);
                }
            }
            if guard.len() >= self.max_samples {
                self.finished.store(true, Ordering::Relaxed);
            }
        }
        let rms = (sum_squares as f64 / frame.len() as f64).sqrt() as f32;
        if let Ok(mut value) = self.level.lock() {
            *value = (rms / 32768.0).sqrt().clamp(0.0, 1.0);
        }
        if let Ok(mut value) = self.peak.lock() {
            *value = value.max((frame_peak as f32 / 32768.0).clamp(0.0, 1.0));
        }
        self.captured.fetch_add(frame.len() as u32, Ordering::Relaxed);
        !self.finished.load(Ordering::Relaxed)
    }

    fn close(&self) {
        self.finished.store(true, Ordering::Relaxed);
        if let Ok(mut value) = self.level.lock() {
            *value = 0.0;
        }
    }
}

/// Live microphone capture. Linux reads PCM from the PipeWire/PulseAudio
/// command-line clients so no ALSA headers are linked; Windows and macOS use
/// the native audio APIs (WASAPI, CoreAudio) through cpal.
pub struct Recorder {
    sink: Arc<Sink>,
    capture: backend::Capture,
    sample_rate: u32,
    started: Instant,
    device: Option<String>,
}

impl Recorder {
    pub fn start(device: Option<&str>, max_seconds: u32) -> AppResult<Self> {
        let device = device.filter(|name| !name.trim().is_empty());
        backend::check_device(device)?;
        let max_seconds = max_seconds.clamp(1, 600) as usize;
        let (capture, sample_rate, sink, name) = backend::start(device, max_seconds)?;
        let mut recorder = Self {
            sink,
            capture,
            sample_rate,
            started: Instant::now(),
            device: device.map(str::to_string),
        };

        // Verify that audio is actually flowing before reporting "recording".
        // A dead or permission-blocked device fails here instead of producing
        // an empty transcript later.
        let deadline = Instant::now() + Duration::from_millis(2500);
        while Instant::now() < deadline {
            if recorder.sink.captured.load(Ordering::Relaxed) > 0 {
                return Ok(recorder);
            }
            if recorder.sink.finished.load(Ordering::Relaxed) {
                break;
            }
            if let Some(status) = recorder.capture.exited() {
                recorder.capture.halt();
                return Err(AppError::new(
                    "audio_capture_failed",
                    format!("The audio backend ({name}) stopped before any audio arrived."),
                )
                .hint("Check that the selected microphone is available and not in use exclusively by another app.")
                .detail(status));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        recorder.capture.halt();
        Err(AppError::new(
            "audio_capture_failed",
            "No audio arrived from the selected microphone.",
        )
        .hint(backend::NO_AUDIO_HINT)
        .detail(format!("backend {name}, device {}", device.unwrap_or("default"))))
    }

    pub fn level(&self) -> f32 {
        self.sink.level.lock().map(|v| *v).unwrap_or(0.0)
    }

    pub fn peak(&self) -> f32 {
        self.sink.peak.lock().map(|v| *v).unwrap_or(0.0)
    }

    pub fn device(&self) -> Option<&str> {
        self.device.as_deref()
    }

    pub fn duration_ms(&self) -> u64 {
        self.started.elapsed().as_millis() as u64
    }

    pub fn is_finished(&self) -> bool {
        self.sink.finished.load(Ordering::Relaxed)
            || self
                .sink
                .samples
                .lock()
                .map(|s| s.len() >= self.sink.max_samples)
                .unwrap_or(true)
    }

    pub fn stop(mut self) -> AppResult<RecordedAudio> {
        self.capture.halt();
        let captured = self
            .sink
            .samples
            .lock()
            .map(|mut guard| std::mem::take(&mut *guard))
            .unwrap_or_default();
        let samples = to_model_rate(&captured, self.sample_rate);
        if samples.len() < (SAMPLE_RATE as usize / 5) {
            return Err(AppError::new(
                "recording_too_short",
                "The recording was too short to transcribe.",
            )
            .hint("Hold the shortcut slightly longer and speak clearly."));
        }
        Ok(RecordedAudio {
            samples,
            sample_rate: SAMPLE_RATE,
            peak: self.peak(),
        })
    }

    pub fn cancel(mut self) {
        self.capture.halt();
        if let Ok(mut samples) = self.sink.samples.lock() {
            samples.clear();
        }
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.capture.halt();
    }
}

/// whisper.cpp needs 16 kHz; native devices usually run at 44.1/48 kHz.
fn to_model_rate(samples: &[i16], rate: u32) -> Vec<i16> {
    if rate == SAMPLE_RATE {
        return samples.to_vec();
    }
    let floats: Vec<f32> = samples.iter().map(|s| *s as f32 / 32768.0).collect();
    resample(&floats, rate, SAMPLE_RATE)
        .into_iter()
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect()
}

#[cfg(target_os = "linux")]
mod backend {
    use super::*;

    pub const NO_AUDIO_HINT: &str =
        "Pick another input device in Settings → Audio, or check your system sound settings.";

    pub struct Capture {
        child: Child,
        reader: Option<std::thread::JoinHandle<()>>,
    }

    impl Capture {
        pub fn exited(&mut self) -> Option<String> {
            match self.child.try_wait() {
                Ok(Some(status)) => Some(format!("capture process exited with {status}")),
                _ => None,
            }
        }

        pub fn halt(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
            if let Some(reader) = self.reader.take() {
                let _ = reader.join();
            }
        }
    }

    pub fn check_device(device: Option<&str>) -> AppResult<()> {
        let Some(name) = device else { return Ok(()) };
        if command_exists("pactl") {
            let known = list_devices()
                .map(|devices| devices.iter().any(|device| device.id == name))
                .unwrap_or(true);
            if !known {
                return Err(AppError::new(
                    "microphone_unavailable",
                    format!("The microphone \"{name}\" is not available."),
                )
                .hint("Choose another input device in Settings → Audio, or set it back to System default."));
            }
        }
        Ok(())
    }

    pub fn start(
        device: Option<&str>,
        max_seconds: usize,
    ) -> AppResult<(Capture, u32, Arc<Sink>, &'static str)> {
        let sink = Arc::new(Sink::new(SAMPLE_RATE as usize * max_seconds, SAMPLE_RATE));
        let (mut child, name) = spawn_capture(device)?;
        let mut stdout = child.stdout.take().ok_or_else(|| {
            AppError::new(
                "audio_capture_failed",
                "The audio capture process did not provide a stream.",
            )
            .hint("Check your microphone in the system sound settings and try again.")
        })?;
        let thread_sink = Arc::clone(&sink);
        let reader = std::thread::Builder::new()
            .name("dictafox-audio-reader".into())
            .spawn(move || {
                // 100 ms of audio per read keeps the UI responsive.
                let mut buffer = vec![0u8; SAMPLE_RATE as usize / 10 * BYTES_PER_SAMPLE];
                let mut carry: Option<u8> = None;
                let mut frame = Vec::with_capacity(buffer.len() / 2 + 1);
                loop {
                    match stdout.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => {
                            frame.clear();
                            let mut bytes = buffer[..read].iter().copied();
                            if let Some(low) = carry.take() {
                                if let Some(high) = bytes.next() {
                                    frame.push(i16::from_le_bytes([low, high]));
                                }
                            }
                            let rest: Vec<u8> = bytes.collect();
                            let mut chunks = rest.chunks_exact(BYTES_PER_SAMPLE);
                            for chunk in &mut chunks {
                                frame.push(i16::from_le_bytes([chunk[0], chunk[1]]));
                            }
                            carry = chunks.remainder().first().copied();
                            if !thread_sink.ingest(&frame) {
                                break;
                            }
                        }
                    }
                }
                thread_sink.close();
            })
            .map_err(|err| {
                AppError::new("audio_capture_failed", "DictaFox could not start audio capture.")
                    .hint("Restart DictaFox and try again.")
                    .with_detail(err)
            })?;
        Ok((
            Capture {
                child,
                reader: Some(reader),
            },
            SAMPLE_RATE,
            sink,
            name,
        ))
    }

    /// Query PipeWire/PulseAudio for capture sources. DictaFox uses the Pulse
    /// protocol so it works on PulseAudio and on PipeWire's pulse compatibility
    /// layer; monitor sources are listed but marked so the UI can hide them.
    pub fn list_devices() -> AppResult<Vec<AudioDevice>> {
        if !command_exists("pactl") {
            return Err(AppError::new(
                "audio_backend_missing",
                "DictaFox could not find pactl, so audio devices cannot be listed.",
            )
            .hint("Install the PipeWire or PulseAudio utilities: sudo apt install pulseaudio-utils"));
        }

        let default_source = Command::new("pactl")
            .arg("get-default-source")
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .filter(|name| !name.is_empty());

        let json_output = Command::new("pactl")
            .args(["-f", "json", "list", "sources"])
            .output();
        if let Ok(out) = json_output {
            if out.status.success() {
                if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                    let mut devices = Vec::new();
                    if let Some(array) = value.as_array() {
                        for source in array {
                            let Some(name) = source.get("name").and_then(|v| v.as_str()) else {
                                continue;
                            };
                            let description = source
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or(name)
                                .to_string();
                            let is_monitor = source
                                .get("monitor_source")
                                .and_then(|v| v.as_str())
                                .map(|monitor| !monitor.is_empty())
                                .unwrap_or(false)
                                || name.ends_with(".monitor");
                            devices.push(AudioDevice {
                                id: name.to_string(),
                                name: description,
                                is_monitor,
                                is_default: default_source.as_deref() == Some(name),
                            });
                        }
                    }
                    if !devices.is_empty() {
                        return Ok(devices);
                    }
                }
            }
        }

        // Older pactl builds do not support JSON output.
        let out = Command::new("pactl")
            .args(["list", "short", "sources"])
            .output()
            .map_err(|err| {
                AppError::new(
                    "audio_backend_unavailable",
                    "DictaFox could not query the audio server.",
                )
                .hint("Make sure PipeWire or PulseAudio is running, then refresh audio devices.")
                .with_detail(err)
            })?;
        if !out.status.success() {
            return Err(AppError::new(
                "audio_backend_unavailable",
                "The audio server rejected the device query.",
            )
            .hint("Check that PipeWire or PulseAudio is running: systemctl --user status pipewire")
            .detail(String::from_utf8_lossy(&out.stderr).to_string()));
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        let devices = stdout
            .lines()
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let _index = fields.next()?;
                let name = fields.next()?.to_string();
                let driver = fields.next().unwrap_or("").to_string();
                let is_monitor = name.ends_with(".monitor");
                Some(AudioDevice {
                    name: if is_monitor {
                        format!("Monitor of {driver}")
                    } else {
                        name.clone()
                    },
                    is_default: default_source.as_deref() == Some(name.as_str()),
                    id: name,
                    is_monitor,
                })
            })
            .collect::<Vec<_>>();
        if devices.is_empty() {
            return Err(AppError::new(
                "no_microphone",
                "No audio input device was found.",
            )
            .hint("Connect a microphone or headset, then refresh. On a laptop, check that the internal microphone is not disabled in BIOS."));
        }
        Ok(devices)
    }

    pub fn backend_name() -> Option<&'static str> {
        if command_exists("parecord") {
            Some("parecord (PipeWire/PulseAudio)")
        } else if command_exists("pw-record") {
            Some("pw-record (PipeWire)")
        } else {
            None
        }
    }

    fn spawn_capture(device: Option<&str>) -> AppResult<(Child, &'static str)> {
        if command_exists("parecord") {
            let mut cmd = Command::new("parecord");
            cmd.args([
                "--format=s16le",
                "--rate=16000",
                "--channels=1",
                "--raw",
                "--latency-msec=40",
                "--client-name=DictaFox",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
            if let Some(device) = device {
                cmd.arg(format!("--device={device}"));
            }
            let child = cmd.spawn().map_err(|err| {
                AppError::new(
                    "audio_capture_failed",
                    "DictaFox could not start the microphone.",
                )
                .hint("Check that PipeWire or PulseAudio is running and that your user can access the microphone.")
                .with_detail(err)
            })?;
            return Ok((child, "parecord"));
        }

        if command_exists("pw-record") {
            let mut cmd = Command::new("pw-record");
            cmd.args(["--format=s16", "--rate=16000", "--channels=1", "-"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            if let Some(device) = device {
                cmd.arg("--target").arg(device);
            }
            let child = cmd.spawn().map_err(|err| {
                AppError::new(
                    "audio_capture_failed",
                    "DictaFox could not start the microphone.",
                )
                .hint("Check that PipeWire is running.")
                .with_detail(err)
            })?;
            return Ok((child, "pw-record"));
        }

        Err(AppError::new(
            "audio_backend_missing",
            "No supported audio capture tool was found.",
        )
        .hint(
            "Install the PipeWire/PulseAudio utilities: sudo apt install pulseaudio-utils pipewire-bin",
        ))
    }
}

#[cfg(not(target_os = "linux"))]
mod backend {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, SizedSample};
    use std::sync::mpsc;

    #[cfg(target_os = "macos")]
    pub const NO_AUDIO_HINT: &str = "Allow DictaFox in System Settings → Privacy & Security → Microphone, then try again.";
    #[cfg(not(target_os = "macos"))]
    pub const NO_AUDIO_HINT: &str = "Check Settings → Privacy & security → Microphone and allow desktop apps to use the microphone, then try again.";

    /// cpal streams are not `Send` on every platform, so the stream lives on
    /// its own thread and is dropped there when `halt` raises the stop flag.
    pub struct Capture {
        stop: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
        error: Arc<Mutex<Option<String>>>,
    }

    impl Capture {
        pub fn exited(&mut self) -> Option<String> {
            self.error.lock().ok().and_then(|guard| guard.clone())
        }

        pub fn halt(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    pub fn backend_name() -> Option<&'static str> {
        if cfg!(target_os = "macos") {
            Some("CoreAudio")
        } else {
            Some("WASAPI")
        }
    }

    fn find_device(name: Option<&str>) -> AppResult<cpal::Device> {
        let host = cpal::default_host();
        if let Some(name) = name {
            if let Ok(mut devices) = host.input_devices() {
                if let Some(device) = devices.find(|d| d.name().map(|n| n == name).unwrap_or(false)) {
                    return Ok(device);
                }
            }
            return Err(AppError::new(
                "microphone_unavailable",
                format!("The microphone \"{name}\" is not available."),
            )
            .hint("Choose another input device in Settings → Audio, or set it back to System default."));
        }
        host.default_input_device().ok_or_else(|| {
            AppError::new("no_microphone", "No audio input device was found.")
                .hint("Connect a microphone or headset, then refresh.")
        })
    }

    pub fn check_device(device: Option<&str>) -> AppResult<()> {
        find_device(device).map(|_| ())
    }

    pub fn list_devices() -> AppResult<Vec<AudioDevice>> {
        let host = cpal::default_host();
        let default_name = host.default_input_device().and_then(|d| d.name().ok());
        let devices = host.input_devices().map_err(|err| {
            AppError::new(
                "audio_backend_unavailable",
                "DictaFox could not query the audio system.",
            )
            .hint("Check your sound settings, then refresh audio devices.")
            .with_detail(err)
        })?;
        let list: Vec<AudioDevice> = devices
            .filter_map(|device| device.name().ok())
            .map(|name| AudioDevice {
                is_default: default_name.as_deref() == Some(name.as_str()),
                id: name.clone(),
                name,
                is_monitor: false,
            })
            .collect();
        if list.is_empty() {
            return Err(AppError::new(
                "no_microphone",
                "No audio input device was found.",
            )
            .hint("Connect a microphone or headset, then refresh."));
        }
        Ok(list)
    }

    fn build<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        sink: Arc<Sink>,
        error: Arc<Mutex<Option<String>>>,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: SizedSample,
        f32: FromSample<T>,
    {
        let channels = (config.channels as usize).max(1);
        let mut mono: Vec<i16> = Vec::new();
        device.build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                mono.clear();
                for frame in data.chunks(channels) {
                    let sum: f32 = frame.iter().map(|s| f32::from_sample(*s)).sum();
                    let value = (sum / frame.len() as f32).clamp(-1.0, 1.0);
                    mono.push((value * 32767.0) as i16);
                }
                sink.ingest(&mono);
            },
            move |err| {
                if let Ok(mut guard) = error.lock() {
                    *guard = Some(format!("audio stream error: {err}"));
                }
            },
            None,
        )
    }

    pub fn start(
        device: Option<&str>,
        max_seconds: usize,
    ) -> AppResult<(Capture, u32, Arc<Sink>, &'static str)> {
        let name = backend_name().unwrap_or("native");
        let device = find_device(device)?;
        let supported = device.default_input_config().map_err(|err| {
            AppError::new(
                "audio_capture_failed",
                "DictaFox could not read the microphone's audio format.",
            )
            .hint(NO_AUDIO_HINT)
            .with_detail(err)
        })?;
        let rate = supported.sample_rate().0;
        let format = supported.sample_format();
        let config: cpal::StreamConfig = supported.into();
        let sink = Arc::new(Sink::new(rate as usize * max_seconds, rate));
        let stop = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();

        let thread = {
            let sink = Arc::clone(&sink);
            let stop = Arc::clone(&stop);
            let error = Arc::clone(&error);
            std::thread::Builder::new()
                .name("dictafox-audio-stream".into())
                .spawn(move || {
                    let stream = match format {
                        cpal::SampleFormat::F32 => build::<f32>(&device, &config, Arc::clone(&sink), error),
                        cpal::SampleFormat::I16 => build::<i16>(&device, &config, Arc::clone(&sink), error),
                        cpal::SampleFormat::U16 => build::<u16>(&device, &config, Arc::clone(&sink), error),
                        cpal::SampleFormat::I32 => build::<i32>(&device, &config, Arc::clone(&sink), error),
                        cpal::SampleFormat::F64 => build::<f64>(&device, &config, Arc::clone(&sink), error),
                        cpal::SampleFormat::U8 => build::<u8>(&device, &config, Arc::clone(&sink), error),
                        other => {
                            let _ = ready_tx.send(Err(format!("unsupported sample format {other:?}")));
                            return;
                        }
                    };
                    let stream = match stream {
                        Ok(stream) => stream,
                        Err(err) => {
                            let _ = ready_tx.send(Err(err.to_string()));
                            return;
                        }
                    };
                    if let Err(err) = stream.play() {
                        let _ = ready_tx.send(Err(err.to_string()));
                        return;
                    }
                    let _ = ready_tx.send(Ok(()));
                    while !stop.load(Ordering::Relaxed) && !sink.finished.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    drop(stream);
                    sink.close();
                })
                .map_err(|err| {
                    AppError::new("audio_capture_failed", "DictaFox could not start audio capture.")
                        .hint("Restart DictaFox and try again.")
                        .with_detail(err)
                })?
        };

        let ready = ready_rx
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| Err("the audio stream did not start".into()));
        let mut capture = Capture {
            stop,
            thread: Some(thread),
            error,
        };
        if let Err(detail) = ready {
            capture.halt();
            return Err(AppError::new(
                "audio_capture_failed",
                "DictaFox could not start the microphone.",
            )
            .hint(NO_AUDIO_HINT)
            .detail(detail));
        }
        Ok((capture, rate, sink, name))
    }
}

pub use backend::{backend_name, list_devices};

pub fn default_input_device() -> Option<String> {
    list_devices().ok().and_then(|devices| {
        devices
            .iter()
            .find(|d| d.is_default && !d.is_monitor)
            .or_else(|| devices.iter().find(|d| !d.is_monitor))
            .map(|d| d.id.clone())
    })
}

/// Linear resampling is enough for speech models; whisper.cpp requires 16 kHz.
pub fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let output_len = ((samples.len() as f64) / ratio).floor() as usize;
    let mut output = Vec::with_capacity(output_len);
    for index in 0..output_len {
        let position = index as f64 * ratio;
        let left = position.floor() as usize;
        let right = (left + 1).min(samples.len() - 1);
        let fraction = (position - left as f64) as f32;
        output.push(samples[left] * (1.0 - fraction) + samples[right] * fraction);
    }
    output
}

pub fn to_mono(channels: &[f32], channel_count: u16) -> Vec<f32> {
    if channel_count <= 1 {
        return channels.to_vec();
    }
    channels
        .chunks(channel_count as usize)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

pub fn write_wav(path: &Path, samples: &[i16], sample_rate: u32) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)?;
    for sample in samples {
        writer.write_sample(*sample)?;
    }
    writer.finalize()?;
    Ok(())
}

pub fn read_wav(path: &Path) -> AppResult<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let mut mono = Vec::new();
    match spec.sample_format {
        hound::SampleFormat::Int => {
            if spec.bits_per_sample <= 16 {
                for sample in reader.samples::<i16>() {
                    mono.push(sample? as f32 / 32768.0);
                }
            } else {
                let max = (1i64 << (spec.bits_per_sample - 1)) as f32;
                for sample in reader.samples::<i32>() {
                    mono.push(sample? as f32 / max);
                }
            }
        }
        hound::SampleFormat::Float => {
            for sample in reader.samples::<f32>() {
                mono.push(sample?);
            }
        }
    }
    Ok((to_mono(&mono, spec.channels), spec.sample_rate))
}

/// Convert captured PCM to the float buffer whisper.cpp expects, applying
/// user gain and peak normalisation only when the recording is quiet.
pub fn prepare_samples(samples: &[i16], gain: f32) -> Vec<f32> {
    let gain = gain.clamp(0.5, 3.0);
    let mut floats: Vec<f32> = samples
        .iter()
        .map(|sample| (*sample as f32 / 32768.0) * gain)
        .collect();
    let peak = floats.iter().fold(0.0f32, |acc, s| acc.max(s.abs()));
    if peak > 0.0001 && peak < 0.5 {
        let factor = 0.85 / peak;
        for sample in &mut floats {
            *sample = (*sample * factor).clamp(-1.0, 1.0);
        }
    } else {
        for sample in &mut floats {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
    floats
}

/// Energy-based voice activity trimming: removes leading and trailing silence
/// while keeping 200 ms of padding so whisper does not clip word onsets.
pub fn trim_silence(samples: &[i16], sample_rate: u32) -> Vec<i16> {
    let frame = (sample_rate as usize / 50).max(1); // 20 ms
    if samples.len() < frame * 5 {
        return samples.to_vec();
    }
    let peak = samples
        .iter()
        .fold(0i16, |acc, s| acc.max(s.saturating_abs())) as i32;
    if peak == 0 {
        return Vec::new();
    }
    let threshold = ((peak as f32) * 0.03).max(260.0) as i32;
    let frame_is_voice = |start: usize| -> bool {
        let end = (start + frame).min(samples.len());
        samples[start..end]
            .iter()
            .any(|s| (*s as i32).abs() > threshold)
    };
    let mut first_voice = None;
    let mut last_voice = None;
    let mut start = 0;
    while start < samples.len() {
        if frame_is_voice(start) {
            if first_voice.is_none() {
                first_voice = Some(start);
            }
            last_voice = Some((start + frame).min(samples.len()));
        }
        start += frame;
    }
    let (Some(first), Some(last)) = (first_voice, last_voice) else {
        return Vec::new();
    };
    let padding = sample_rate as usize / 5;
    let start = first.saturating_sub(padding);
    let end = (last + padding).min(samples.len());
    samples[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip_preserves_signal() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wav");
        let samples: Vec<i16> = (0..1600).map(|i| (i as i16 * 20) - 8000).collect();
        write_wav(&path, &samples, SAMPLE_RATE).unwrap();
        let (floats, rate) = read_wav(&path).unwrap();
        assert_eq!(rate, SAMPLE_RATE);
        assert_eq!(floats.len(), samples.len());
        assert!((floats[5] - samples[5] as f32 / 32768.0).abs() < 0.0001);
    }

    #[test]
    fn silence_is_trimmed_with_padding() {
        let mut samples = vec![0i16; SAMPLE_RATE as usize]; // 1 s silence
        samples.extend(std::iter::repeat(6000i16).take(SAMPLE_RATE as usize / 2)); // 0.5 s tone
        samples.extend(std::iter::repeat(0i16).take(SAMPLE_RATE as usize)); // 1 s silence
        let trimmed = trim_silence(&samples, SAMPLE_RATE);
        let trimmed_ms = trimmed.len() as u64 * 1000 / SAMPLE_RATE as u64;
        assert!((700..=1100).contains(&trimmed_ms), "trimmed {trimmed_ms} ms");
        assert!(trimmed.iter().any(|s| *s != 0));
    }

    #[test]
    fn fully_silent_audio_trims_to_nothing() {
        let samples = vec![0i16; SAMPLE_RATE as usize];
        assert!(trim_silence(&samples, SAMPLE_RATE).is_empty());
    }

    #[test]
    fn quiet_recordings_are_normalised_and_clipped() {
        let samples: Vec<i16> = (0..1000).map(|i| if i % 2 == 0 { 1000 } else { -1000 }).collect();
        let floats = prepare_samples(&samples, 1.0);
        let peak = floats.iter().fold(0.0f32, |a, s| a.max(s.abs()));
        assert!((peak - 0.85).abs() < 0.01);
        let loud = prepare_samples(&[i16::MAX, i16::MIN], 2.0);
        assert!(loud.iter().all(|s| s.abs() <= 1.0));
    }

    #[test]
    fn resampling_changes_length_proportionally() {
        let input = vec![0.25f32; 48_000];
        let output = resample(&input, 48_000, 16_000);
        assert!((output.len() as i64 - 16_000).abs() < 2);
        assert!(output.iter().all(|s| (*s - 0.25).abs() < 0.001));
    }

    #[test]
    fn stereo_downmix_averages_channels() {
        let stereo = vec![1.0, -1.0, 0.5, 0.5];
        assert_eq!(to_mono(&stereo, 2), vec![0.0, 0.5]);
    }

    #[test]
    fn capture_reports_a_clear_error_for_an_unknown_device() {
        if backend_name().is_none() {
            eprintln!("skipping: no PipeWire/PulseAudio capture tool in this environment");
            return;
        }
        let result = Recorder::start(Some("dictafox-nonexistent-input"), 3);
        let err = result.err().expect("unknown device must fail, not hang");
        assert!(
            err.code == "audio_capture_failed"
                || err.code == "audio_backend_unavailable"
                || err.code == "microphone_unavailable",
            "unexpected error: {err:?}"
        );
    }
}
