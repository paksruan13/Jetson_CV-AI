use crate::audio::AudioMetrics;
use std::io::{self, Write};

pub struct AudioMeter {
    frame_count: u64,
    bar_length: usize,
    initialized: bool,
}

impl AudioMeter {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            bar_length: 40,
            initialized: false,
        }
    }

    pub fn display(&mut self, metrics: &AudioMetrics) {
        self.frame_count += 1;
        if !self.initialized {
            println!();
            self.initialized = true;
        }
        let normalized_level = (metrics.rms * 100.0).min(1.0);
        let filled_length = (normalized_level * self.bar_length as f32) as usize;
        let bar = self.create_bar(filled_length);
        
        let signal_strength = self.get_signal_strength(metrics.db);
        print!("\x1b[2K\r");
        print!("Audio: [{}] RMS:{:.3} {:.0}dB | {} |",
               bar,
               metrics.rms,
               metrics.db,
               signal_strength
            );

        io::stdout().flush().unwrap();
    }

    fn create_bar(&self, filled_length: usize) -> String {
        let filled_char = "█";
        let empty_char = " ";
        filled_char.repeat(filled_length) + &empty_char.repeat(self.bar_length - filled_length)
    }

    fn get_signal_strength(&self, db: f32) -> &'static str {
        match db {
            db if db > -20.0 => "LOUDDDDD",
            db if db > -35.0 => "GOOD",
            db if db > -50.0 => "LOW",
            _ => "SILENCE"
        }
    }
}