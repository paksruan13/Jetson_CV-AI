use std::collections::VecDeque;
/// Noise Gate filter 
/// 
/// -Convert sample amplitude to dB
/// -Compare to threshold
/// -Open gate if above threhold, else close
/// -Apply envelop to prevent clicks

#[derive(Debug, Clone)]
pub struct NoiseGate {
    /// Set db threshold
    threshold_db: f32,
    attack_samples: usize,
    release_samples: usize,
    state: GateState,
    envelope: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum GateState {
    Open,
    Closed,
}

impl NoiseGate {
    ///Create a new noise gate filter
    /// Args:
    /// - threshold_db: dB level to open gate
    /// - attack_ms: time to open gate (ms)
    /// - release_ms: time to close gate (ms)
    /// -sample_rate: audio sample rate (Hz)
    pub fn new(threshold_db: f32, attack_ms: f32, release_ms: f32, sample_rate: f32) -> Self {
        let attack_samples = (attack_ms * sample_rate / 1000.0) as usize; // convert ms -> sample count
        let release_samples = (release_ms * sample_rate / 1000.0) as usize; //converting for samples/sec
        Self {
            threshold_db,
            attack_samples: attack_samples.max(1),
            release_samples: release_samples.max(1),
            state: GateState::Closed,
            envelope: 0.0,
        }
    }

    /// Process audio samples thru noise gate
    /// Args:
    /// - Samples: mutable slice of f32 audio sample 
    pub fn process(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            // Convert sample amp to db
            // db = 20 * log10(amp), add 1e-10 to prevent log10(0) -> -inf
            let sample_db = 20.0 * sample.abs().max(1e-10).log10();

            match self.state {
                GateState::Closed => {
                    // check if signal exceeds threshold
                    if sample_db > self.threshold_db {
                        self.state = GateState::Open; //Open if exceeds
                        self.envelope = 0.0; // Start attack from 0
                    } 
                }
                GateState::Open => {
                    if sample_db < self.threshold_db {
                        self.state = GateState::Closed;
                        self.envelope = 1.0; // Start release from 1
                    }
                }
            }
            // Calc target envelope based on state
            let target = if self.state == GateState::Open { 1.0 } else { 0.0 };

            // Calculate envelope slew rate
            let rate = if target > self.envelope {
                1.0 / self.attack_samples as f32
            } else {
                1.0 / self.release_samples as f32
            };

            self.envelope += (target - self.envelope) * rate;
            *sample *= self.envelope;
        }
    }

    pub fn reset(&mut self) {
        self.state = GateState::Closed;
        self.envelope = 0.0;
    }
}

/// Audio Normalizer
/// 
/// Normalize volume across different speakers
/// Algo:
/// -Calc rolling RMS l evel over window
/// -Computer gain needed to reach target level
/// -Apply smoothed gain
/// -Limit output range
#[derive(Debug, Clone)]
pub struct Normalizer {
    target_level_db: f32,
    window_size: usize,
    buffer: VecDeque<f32>, //VecDeque for efficient push/pop operations
    current_gain: f32,
    adaptation_rate: f32,
}

impl Normalizer {
    /// Create new normalizer
    /// Args:
    /// - target_level_db
    /// - window_ms
    /// -sample_rate
    pub fn new(target_level_db: f32, window_ms: f32, sample_rate: f32) -> Self {
        //Convert window ms to sample count
        let window_size = (window_ms * sample_rate / 1000.0) as usize;
        Self {
            target_level_db,
            window_size,
            //Pre-allocate buffer capacity to avoid reallocation
            buffer: VecDeque::with_capacity(window_size),
            current_gain: 1.0, //Init gain
            adaptation_rate: 0.01, // 1% change per sample
        }
    }
    /// Process audio samples thru normalized
    /// Args:
    /// - samples; mutable slice of f32 audio samples
    pub fn process(&mut self, samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            //Add current sample to RMS buffer

            //Remove oldest sample if buffer exceeds window size
            self.buffer.push_back(sample.abs());
            if self.buffer.len() > self.window_size {
                self.buffer.pop_front();
            } //This is nice deque in action ^ 
            let rms: f32 = ( //average loudness
                self.buffer
                    .iter() // Iterate over buffer
                    .map(|&x| x * x) // Squre each value
                    .sum::<f32>() // sum all squared values
                    / self.buffer.len() as f32 //divide by tot count
            ).sqrt(); // take sqrt

            // Only calculate gain if RMS is above noise floor
            if rms > 1e-10 {
                let current_db = 20.0 * rms.log10(); //Power is propotional to Amplitude^2
                let target_gain = 10.0_f32.powf((self.target_level_db - current_db) / 20.0); // inverse dB formula for linear gain
                self.current_gain += (target_gain - self.current_gain) * self.adaptation_rate; //Exponential moving average for smooth gain
            }
            *sample *= self.current_gain; //apply to sample
            *sample = sample.clamp(-1.0, 1.0); //hard limit to prevent clipping
        }
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.current_gain = 1.0;
    }
}

//═══════════════════════════════════════════════════════════════════════════
// UNIT TESTS, delete later, temp here for now
//═══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_gate_removes_quiet_audio() {
        // Create gate with -40dB threshold
        let mut gate = NoiseGate::new(-40.0, 10.0, 50.0, 44100.0);
        
        // Very quiet samples (way below threshold)
        // These are around -60dB, well below -40dB threshold
        let mut samples = vec![0.001, 0.002, 0.001, 0.002, 0.001];

        gate.process(&mut samples);

        // All samples should be heavily attenuated (close to zero)
        // Gate should remain closed for samples this quiet
        assert!(samples.iter().all(|&s| s.abs() < 0.01));
    }

    #[test]
    fn test_noise_gate_passes_loud_audio() {
        // Create gate with -40dB threshold and VERY fast attack/release
        // Fast parameters ensure gate opens quickly during test
        let mut gate = NoiseGate::new(-40.0, 1.0, 10.0, 44100.0);
        
        // Loud samples (well above threshold at ~-6dB)
        // Need enough samples for envelope to ramp up
        let mut samples = vec![0.5; 500];  // 500 samples at 0.5 amplitude

        gate.process(&mut samples);

        // After processing 500 samples, envelope should have ramped up
        // Check that LATER samples (after attack time) pass through
        let samples_after_attack = &samples[100..];  // Skip first 100 samples
        assert!(
            samples_after_attack.iter().any(|&s| s.abs() > 0.3),
            "Expected some samples > 0.3 after attack period, but max was: {}",
            samples_after_attack.iter().map(|&s| s.abs()).fold(0.0f32, f32::max)
        );
    }

    #[test]
    fn test_normalizer_adjusts_levels() {
        // Create normalizer targeting -20dB
        let mut normalizer = Normalizer::new(-20.0, 100.0, 44100.0);
        
        // Quiet samples (will need significant gain boost)
        // Start with ~-40dB signal, normalizer should boost to -20dB
        let mut samples = vec![0.01; 5000];  // Need many samples for adaptation

        normalizer.process(&mut samples);

        // Check that gain increased over time
        // Compare early samples vs late samples
        let early_avg = samples[0..100].iter().sum::<f32>() / 100.0;
        let late_avg = samples[4900..5000].iter().sum::<f32>() / 100.0;
        
        assert!(
            late_avg > early_avg,
            "Expected normalizer to increase gain over time. Early avg: {:.4}, Late avg: {:.4}",
            early_avg, late_avg
        );
        
        // Also verify some samples are significantly boosted
        assert!(
            samples[4900..].iter().any(|&s| s > 0.05),
            "Expected some late samples > 0.05 after normalization"
        );
    }

    #[test]
    fn test_normalizer_prevents_clipping() {
        // Create normalizer (will try to boost quiet input)
        let mut normalizer = Normalizer::new(-20.0, 100.0, 44100.0);
        
        // Already loud samples (close to clipping at 0dB)
        let mut samples = vec![0.9; 1000];

        normalizer.process(&mut samples);

        // Hard limiter should prevent any sample exceeding [-1.0, 1.0]
        assert!(
            samples.iter().all(|&s| s.abs() <= 1.0),
            "Found sample exceeding clipping limit: {:?}",
            samples.iter().find(|&&s| s.abs() > 1.0)
        );
    }

    #[test]
    fn test_noise_gate_envelope_smoothness() {
        // Test that envelope ramps smoothly (no sudden jumps)
        let mut gate = NoiseGate::new(-40.0, 10.0, 50.0, 44100.0);
        
        // Start with silence, then loud signal
        let mut samples = vec![0.001; 100];  // Quiet start
        samples.extend(vec![0.5; 500]);      // Then loud
        
        gate.process(&mut samples);
        
        // Check for smooth transition (no big jumps between adjacent samples)
        for i in 1..samples.len() {
            let jump = (samples[i] - samples[i-1]).abs();
            assert!(
                jump < 0.2,  // Allow some ramp but not instant jumps
                "Found large jump at sample {}: {:.4} -> {:.4} (jump: {:.4})",
                i, samples[i-1], samples[i], jump
            );
        }
    }

    #[test]
    fn test_normalizer_target_level() {
        // Verify normalizer actually reaches target level
        let mut normalizer = Normalizer::new(-20.0, 200.0, 44100.0);
        
        // Consistent input signal
        let mut samples = vec![0.05; 10000];  // Long buffer for convergence
        
        normalizer.process(&mut samples);
        
        // Calculate final RMS after normalization
        let final_samples = &samples[9000..10000];
        let rms: f32 = (
            final_samples.iter()
                .map(|&x| x * x)
                .sum::<f32>()
                / final_samples.len() as f32
        ).sqrt();
        
        let final_db = 20.0 * rms.log10();
        
        // Should be close to target -20dB (within 3dB tolerance)
        assert!(
            (final_db - (-20.0)).abs() < 3.0,
            "Expected final level near -20dB, got {:.2}dB",
            final_db
        );
    }
}