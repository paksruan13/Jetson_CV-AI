use super::traits::{AudioWriter, RecordingInfo};
use hound::{WavWriter, WavSpec};
use std::path::PathBuf;
use std::fs;
use chrono::Local;

//Implementation, like setting up repo for audio data to WAV files
/// WAV file writer with auto timestamped filenames and daily rotation
/// -Manual Control via start_writing and finish_writing FNs
/// -Auto timestamp filenames
pub struct WavFileWriter {
    ///Active WAV writer wrapped in option for safe state management
    writer: Option<WavWriter<std::io::BufWriter<std::fs::File>>>, 
    current_file: Option<PathBuf>, //track current file path
    sample_count: u64,
    sample_rate: u32,
    output_dir: PathBuf, //Dir where WAV files are saved.
    current_date: Option<String>, //Dated in string format YYYYMMDD
}

impl WavFileWriter {
    ///Create a new WAV file writer with specified output dir
    /// # Arguments
    /// - `output_dir`: Directory where WAV files will be saved (ex: "./recordings")
    pub fn new(output_dir: impl Into<PathBuf>) -> Self {
        Self {
            writer: None,
            current_file: None,
            sample_count: 0,
            sample_rate: 44100, //Default state
            output_dir: output_dir.into(),
            current_date: None,
        }
    }

    fn generate_filename() -> String {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        format!("audio_{}.wav", timestamp)
    }

    fn get_current_date() -> String {
        Local::now().format("%Y%m%d").to_string()
    }

    ///Check if we crossed day boundary
    /// **Returns**: true if date changed since file was created
    fn should_rotate(&self) -> bool {
        if let Some(ref date) = self.current_date {
            date != &Self::get_current_date()
        } else {
            false
        }
    }

    ///Auto rotate to new file if day boundary crossed
    fn check_rotation(&mut self, channels: u16) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_writing() && self.should_rotate() { //called auto by these Args to ensure cont. recording
            println!("Day Boundary crossed, Rotating file.");
            let info = self.finish_writing()?;
            if let Some(info) = info {
                println!("Closed: {:?} ({:.2}s)", info.file_path, info.duration_seconds);
            }

            self.start_writing(self.sample_rate, channels)?;
        }
        Ok(())
    }
}

impl AudioWriter for WavFileWriter {
    type Error = Box<dyn std::error::Error>;
    fn start_writing(&mut self, sample_rate: u32, channels: u16) -> Result<(), Self::Error>{
        if self.is_writing() {
            return Err("Already writing".into());
        }
        
        fs::create_dir_all(&self.output_dir)?; //Create output dir if it doesn't exist

        let filename = Self::generate_filename();
        let path = self.output_dir.join(filename);

        let spec = WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 32,              //Using 32 bits for float samples
            sample_format: hound::SampleFormat::Float,
        };

        let writer = WavWriter::create(&path, spec)?; //Creating WAV writer..open con
        //Store state
        self.writer = Some(writer);
        self.current_file = Some(path.clone());
        self.sample_count = 0;
        self.sample_rate = sample_rate;
        self.current_date = Some(Self::get_current_date());

        println!("Started Recording: {:?}", path);
        Ok(())

    }

    ///Write audio samples to file with auto rotation check
    fn write_samples(&mut self, samples: &[f32]) -> Result<(), Self::Error> {
        self.check_rotation(1)?;
        if let Some(ref mut writer) = self.writer {
            for &sample in samples {
                writer.write_sample(sample)?;
            }
            self.sample_count += samples.len() as u64;
            Ok(())
        } else {
            Err("Not currently writing to a file".into())
        }
    }

    fn finish_writing(&mut self) -> Result<Option<RecordingInfo>, Self::Error> {
        if let (Some(writer), Some(file_path)) = (self.writer.take(), self.current_file.take()) {
            writer.finalize()?;
            let file_size = fs::metadata(&file_path)?.len();
            let duration_seconds = self.sample_count as f64 / self.sample_rate as f64;
            let info = RecordingInfo {
                file_path,
                duration_seconds,
                file_size_bytes: file_size,
                sample_rate: self.sample_rate,
                channels: 1,
            };
            println!("Recording Finished: {:.2}s, {} bytes", info.duration_seconds, info.file_size_bytes);
            self.sample_count = 0; //Reset counter
            self.current_date = None; //Reset date
            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    fn is_writing(&self) -> bool {
        self.writer.is_some()
    }
}

impl Drop for WavFileWriter {
    fn drop(&mut self) {
        if self.is_writing() {
            if let Err(e) = self.finish_writing() {
                eprintln!("Error finalizing WAV file on drop: {}", e);
            }
        }
    }
}