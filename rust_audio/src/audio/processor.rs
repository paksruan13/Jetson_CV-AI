use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, StreamConfig};
use std::sync::{Arc, Mutex};
use super::metrics::AudioMetrics;
use super::filters::{NoiseGate, Normalizer};

pub struct AudioProcessor {
    device: Device,
    config: StreamConfig,
    metrics: Arc<Mutex<AudioMetrics>>,
    noise_gate: NoiseGate,
    normalizer: Normalizer,
}

impl AudioProcessor {
    /// Create a new audio processor with shared metrics
    /// preset before applying in fn start
    pub fn new(metrics: Arc<Mutex<AudioMetrics>>) -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or("No input device available")?;
        println!("Using input device: {}", device.name()?);

        // Get default input config 
        let config = device.default_input_config()?;
        let sample_rate = config.sample_rate().0 as f32;
        println!("Audio config: {:?}", config);
        let noise_gate = NoiseGate::new(
            -40.0,
            10.0,
            100.0,
            sample_rate,
        );
        println!("NoiseGate: -40dB threshold, 10ms attack, 100ms release");
        let normalizer = Normalizer::new(
            -20.0,
            200.0,
            sample_rate,
        );
        println!("Normalizer: -20dB target, 200ms RMS window");
        Ok(Self {
            device,
            config: config.into(),
            metrics,
            noise_gate,
            normalizer,
        })
    }

    ///Start audio capture and processing
    /// **
    /// -Builds audio input with callback
    /// -callback exe on audio thread (low latency)
    /// -updates shared metrics on every audio buffer
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Clone Arc for audio callback
        let metrics_clone = Arc::clone(&self.metrics);
        let mut noise_gate = self.noise_gate.clone();
        let mut normalizer = self.normalizer.clone();
        let stream = self.device.build_input_stream(
            &self.config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mut samples = data.to_vec(); //create mutable copy for filter processing
                noise_gate.process(&mut samples); //apply noise gate
                normalizer.process(&mut samples); //apply normalization
                let sum_squares: f32 = data.iter().map(|&x| x * x).sum();
                let rms = (sum_squares / data.len() as f32).sqrt();
                let peak = data.iter().map(|&x| x.abs()).fold(0.0f32, f32::max);
                // Convert RMS to dbs
                // Formula: dB = 20 * log10(RMS)
                // Adding 1e-10 prevents log10(0) = -infinity
                let db = 20.0 * rms.max(1e-10).log10();

                if let Ok(mut metrics) = metrics_clone.lock() {
                    metrics.update(rms, peak, db);
                }
            },
            |err| {
                eprintln!("Audio stream error: {}", err);
            },
            None,
        )?;
        stream.play()?;
        println!("Audio processing started.");

        //Keep stream alive by moving into infinite loop
        // Stream will drop once processor is dropped
        std::mem::forget(stream);
        Ok(())
    }
}