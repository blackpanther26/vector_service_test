use reqwest::blocking::Client;
use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::os::unix::fs::FileTypeExt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const PIPE_PATH: &str = "/tmp/vector_pipe";

#[derive(Serialize)]
struct VectorizationRequest {
    text: String,
    pooling_strategy: String,
}

#[derive(Deserialize)]
pub struct VectorResponse {
    pub text: String,
    pub vector: Vec<f32>,
}

pub fn vectorise(input: &str, pooling_strategy: &str, base_url: &str) -> VectorResponse {
    let client = Client::new();
    let response = client
        .post(&format!("{}/vectorize/", base_url))
        .json(&VectorizationRequest {
            text: input.to_string(),
            pooling_strategy: pooling_strategy.to_string(),
        })
        .send()
        .expect("Failed to send request");

    response
        .json::<VectorResponse>()
        .expect("Failed to parse vector response")
}

// First, let's modify the read_from_named_pipe function to accept a shutdown signal
pub fn read_from_named_pipe(shutdown: Arc<AtomicBool>) {
    while !shutdown.load(Ordering::Relaxed) {
        // Your existing pipe reading logic here
        // Ensure the named pipe exists
        if !std::path::Path::new(PIPE_PATH).exists() {
            eprintln!("Named pipe does not exist at {}", PIPE_PATH);
            return;
        }

        // Check if the file is a named pipe
        let metadata =
            std::fs::metadata(PIPE_PATH).expect("Unable to fetch metadata for the named pipe");
        if !metadata.file_type().is_fifo() {
            eprintln!("The path is not a named pipe");
            return;
        }

        // Open the named pipe for reading
        let pipe = File::open(PIPE_PATH).expect("Failed to open named pipe");
        let reader = BufReader::new(pipe);

        // Continuously read from the pipe
        println!("Listening for vectors on named pipe: {}", PIPE_PATH);
        for line in reader.lines() {
            match line {
                Ok(json_line) => {
                    // Parse JSON data from the named pipe
                    let parsed: Result<HashMap<String, Vec<f32>>, _> =
                        serde_json::from_str(&json_line);
                    match parsed {
                        Ok(data) => {
                            if let Some(vector) = data.get("vector") {
                                println!("Received vector: {:?}", vector);
                                // Perform validation
                                if vector.iter().all(|v| v.is_finite()) {
                                    println!("Vector is valid!");
                                } else {
                                    println!("Invalid vector received!");
                                }
                            } else {
                                println!("No vector found in the data!");
                            }
                        }
                        Err(e) => eprintln!("Failed to parse JSON from pipe: {}", e),
                    }
                }
                Err(e) => eprintln!("Error reading from named pipe: {}", e),
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
}
