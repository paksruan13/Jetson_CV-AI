pub mod audio;
pub mod display;
pub mod ar;

pub use audio::{AudioMetrics, AudioProcessor, WavFileWriter};
pub use audio::filters::{NoiseGate, Normalizer};
pub use display::AudioMeter;
pub use ar::{ARBridgeServer, ARFrame};

// python binding via PyO3
use pyo3::prelude::*;
use pyo3::types::PyBytes;

#[pyclass]
pub struct PyNoiseGate {
    inner: NoiseGate,
}

#[pymethods]
impl PyNoiseGate {
    #[new]
    fn new(threshold_db: f32, attack_ms: f32, release_ms: f32, sample_rate: f32) -> Self {
        Self {
            inner: NoiseGate::new(threshold_db, attack_ms, release_ms, sample_rate),
        }
    }

    fn process(&mut self, py: Python, samples: &PyBytes) -> PyResult<PyObject> {
        let bytes = samples.as_bytes();
        //convert bytes to f32 vector
        let mut float_samples: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        // apply rust noise gate filter
        self.inner.process(&mut float_samples);
        //convert back to bytes
        let result_bytes: Vec<u8> = float_samples
            .iter()
            .flat_map(|&f| f.to_le_bytes())
            .collect();
        Ok(PyBytes::new(py, &result_bytes).into())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}

// Python wrapper for normalizer filter
#[pyclass]
pub struct PyNormalizer {
    inner: Normalizer,
}

#[pymethods]
impl PyNormalizer {
    #[new]
    fn new(target_level_db: f32, window_ms: f32, sample_rate: f32) -> Self {
        Self {
            inner: Normalizer::new(target_level_db, window_ms, sample_rate),
        }
    }

    fn process(&mut self, py: Python, samples: &PyBytes) -> PyResult<PyObject> {
        let bytes = samples.as_bytes();

        let mut float_samples: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();

        self.inner.process(&mut float_samples);

        let result_bytes: Vec<u8> = float_samples
            .iter()
            .flat_map(|&f| f.to_le_bytes())
            .collect();

        Ok(PyBytes::new(py, &result_bytes).into())
    }

    fn reset(&mut self) {
        self.inner.reset();
    }
}

//Python module definiton: 
#[pymodule]
fn p1(_py: Python, m:&PyModule) -> PyResult<()> {
    m.add_class::<PyNoiseGate>()?;
    m.add_class::<PyNormalizer>()?;
    Ok(());
}