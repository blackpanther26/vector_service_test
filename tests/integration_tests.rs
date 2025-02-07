use serde_json::json;
use std::fs;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use vector_service_test::read_from_named_pipe;
use vector_service_test::vectorise;

#[cfg(test)]
mod tests {
    use super::*;
    use mockito;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn setup() {
        INIT.call_once(|| {
            // Create named pipe if it doesn't exist
            if !std::path::Path::new("/tmp/vector_pipe").exists() {
                use std::process::Command;
                Command::new("mkfifo")
                    .arg("/tmp/vector_pipe")
                    .status()
                    .expect("Failed to create named pipe");
            }
        });
    }

    #[test]
    fn test_vectorization_service() {
        // Create a mock server
        let mut server = mockito::Server::new();

        // Create the mock response
        let mock_response = json!({
            "text": "test",
            "vector": [0.1, 0.2, 0.3]
        });

        // Set up the mock
        let mock = server
            .mock("POST", "/vectorize/")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(mock_response.to_string())
            .create();

        // Make the actual request
        // Note: You'll need to modify the vectorise function to accept a base_url parameter
        let response = vectorise("test", "mean", &server.url());

        assert_eq!(response.text, "test");
        assert_eq!(response.vector, vec![0.1, 0.2, 0.3]);

        mock.assert();
    }

    #[test]
    fn test_named_pipe_writing() {
        setup();

        // Create shutdown signal
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // Spawn reader thread
        let reader_thread = thread::spawn(move || {
            read_from_named_pipe(shutdown_clone);
        });

        // Give the reader thread time to start
        thread::sleep(Duration::from_millis(100));

        // Write test data to the pipe
        let test_vector = json!({
            "vector": vec![1.0, 2.0, 3.0]
        });

        let mut file = fs::OpenOptions::new()
            .write(true)
            .open("/tmp/vector_pipe")
            .expect("Failed to open pipe for writing");

        writeln!(file, "{}", test_vector.to_string()).expect("Failed to write to pipe");

        // Allow time for processing
        thread::sleep(Duration::from_millis(500));

        // Signal shutdown
        shutdown.store(true, Ordering::Relaxed);

        // Wait for reader thread to finish
        reader_thread.join().unwrap();
    }

    #[test]
    fn test_invalid_vector() {
        setup();

        // Create shutdown signal
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        // Spawn reader thread
        let reader_thread = thread::spawn(move || {
            read_from_named_pipe(shutdown_clone);
        });

        thread::sleep(Duration::from_millis(100));

        // Test with invalid vector
        let invalid_vector = json!({
            "vector": vec![1.0, f32::INFINITY, 3.0]
        });

        let mut file = fs::OpenOptions::new()
            .write(true)
            .open("/tmp/vector_pipe")
            .expect("Failed to open pipe for writing");

        writeln!(file, "{}", invalid_vector.to_string()).expect("Failed to write to pipe");

        thread::sleep(Duration::from_millis(500));

        // Signal shutdown
        shutdown.store(true, Ordering::Relaxed);

        // Wait for reader thread to finish
        reader_thread.join().unwrap();
    }
}
