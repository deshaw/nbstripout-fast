mod stripoutlib;

#[cfg(feature = "extension-module")]
mod python {
    use pyo3::exceptions::PyRuntimeError;
    use pyo3::prelude::*;
    use serde_json;

    use super::stripoutlib;

    /// Strips output from a notebook (string) and returns back a notebook (string)
    #[pyfunction]
    fn stripout(
        contents: String,
        keep_output: bool,
        keep_count: bool,
        extra_keys: Vec<String>,
        drop_empty_cells: bool,
    ) -> PyResult<String> {
        // If rust ever comes up with a PyObject to serde we should accept a
        // notebook object instead. This is cheap and mostly used for testing
        let mut nb: serde_json::Value = serde_json::from_str(&contents).map_err(|e| {
            PyRuntimeError::new_err(format!("JSON was not well-formatted: {:?}", e))
        })?;

        stripoutlib::strip_output(
            &mut nb,
            keep_output,
            keep_count,
            &extra_keys,
            drop_empty_cells,
        )
        .map_err(PyRuntimeError::new_err)?;

        let cleaned_contents = serde_json::to_string_pretty(&nb).map_err(|e| {
            PyRuntimeError::new_err(format!("JSON output was not well-formatted: {:?}", e))
        })?;

        Ok(cleaned_contents)
    }

    /// nbstripout, but in rust!
    #[pymodule]
    fn nbstripout_fast(_py: Python, m: &PyModule) -> PyResult<()> {
        env_logger::init();

        m.add_function(wrap_pyfunction!(stripout, m)?)?;
        Ok(())
    }
}
