use p1::audio::{AudioMetrics, AudioProcessor, WavFileWriter};
use p1::display::AudioMeter;

use std::io::{self, Write};
use std::sync::{Arc, Mutex}; //Thread-safe shraed state
use std::time::Duration;
use std::thread;


fn main() -> Result<(), Box<dyn std::error::Error>> { //Error handling with Result<T, E>
    println!("Starting MERLIN Audio System...");

    //Allow shared auido metrics (thread-safe using Arc<Mutex<T>>)
    // Atomic ref counting (ARC) for shared ownership across threads
    // Mutual exclusion (Ensures only one thread modifies at a time)
    let metrics = Arc::new(Mutex::new(AudioMetrics::new()));
    let metrics_clone = Arc::clone(&metrics);

    let _wav_writer = WavFileWriter::new("./recordings");
    println!("WAV recorder initialized: ./recordings");

    let mut processor: AudioProcessor = AudioProcessor::new(metrics_clone).expect("Failed to create audio processor");

    //Start audio processing in background thread
    let _processor_handle = thread::spawn(move || {
        processor.start().expect("Failed to start audio processing");
    });

    thread::sleep(Duration::from_millis(100)); //Give audio thread time to initialize

    //Display real-time audio emter in main thread
    let mut meter = AudioMeter::new();
    println!("Audio monitoring is LIVE");

    loop {
        //Update display with current audio levels
        let current_metrics = metrics.lock().unwrap().clone();
        meter.display(&current_metrics);

        //Refresh display every 50ms (20fps)
        //Target: < 50ms latency for audio -> display pipeline
        thread::sleep(Duration::from_millis(50));

        //Flush stdout to ensure display updates apear immediately
        io::stdout().flush().unwrap();
    }

}
