use std::path::PathBuf;
///Traits for writing audio data to files

pub trait AudioWriter {
    type Error;
    fn start_writing(&mut self, sample_rate: u32, channels: u16) -> Result<(), Self::Error>;
    fn write_samples(&mut self, sample: &[f32]) -> Result<(), Self::Error>;
    fn finish_writing(&mut self) -> Result<Option<RecordingInfo>, Self::Error>;
    fn is_writing(&self) -> bool;
}
 /// Metadata for a completed recording
#[derive(Debug, Clone)]
pub struct RecordingInfo {
    pub file_path: PathBuf,
    pub duration_seconds: f64,
    pub file_size_bytes: u64,
    pub sample_rate: u32,
    pub channels: u16,
}

impl RecordingInfo {
    pub fn print_summary(&self) {
        println!("Recording Completed:");
        println!("File: {:?}", self.file_path);
        println!("Duration: {:.2}s", self.duration_seconds);
        println!("Size: {} Bytes", self.file_size_bytes);
    }
}
