mod handlers;

use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::Duration;

use clap::Parser;

const BAUD_RATE: u32 = 921_600;
const MAX_COBS_MESSAGE_LENGTH: usize = 1024 * 1024;
const READ_BUFFER_LENGTH: usize = 1024;
const READ_TIMEOUT: Duration = Duration::from_millis(100);

#[derive(Parser)]
struct Arguments {
    port_name: String,
    #[arg(default_value = "output")]
    output_directory: PathBuf,
    #[arg(short = 'n', long)]
    message_limit: Option<u64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = Arguments::parse();
    let port_name = arguments.port_name;
    let output_directory = arguments.output_directory;
    fs::create_dir_all(&output_directory)?;

    let mut port = serialport::new(port_name, BAUD_RATE)
        .timeout(READ_TIMEOUT)
        .open()?;
    let mut encoded = Vec::with_capacity(MAX_COBS_MESSAGE_LENGTH);
    let mut decoded = vec![0; MAX_COBS_MESSAGE_LENGTH];
    let mut received = [0; READ_BUFFER_LENGTH];
    let mut remaining_messages = arguments.message_limit;

    while remaining_messages.is_none_or(|remaining| remaining > 0) {
        match port.read(&mut received) {
            Ok(0) => {}
            Ok(received_length) => {
                for &byte in &received[..received_length] {
                    if receive_byte(byte, &mut encoded, &mut decoded, &output_directory) {
                        if let Some(remaining) = &mut remaining_messages {
                            *remaining -= 1;
                        }
                        if remaining_messages == Some(0) {
                            break;
                        }
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => return Err(error.into()),
        }
    }

    Ok(())
}

fn receive_byte(
    byte: u8,
    encoded: &mut Vec<u8>,
    decoded: &mut [u8],
    output_directory: &Path,
) -> bool {
    if byte == 0 {
        if !encoded.is_empty() {
            let dispatched = match decode_and_dispatch(encoded, decoded, output_directory) {
                Ok(()) => true,
                Err(error) => {
                    eprintln!("Discarded message: {error}");
                    false
                }
            };
            encoded.clear();
            return dispatched;
        }
    } else if encoded.len() < MAX_COBS_MESSAGE_LENGTH {
        encoded.push(byte);
    } else {
        eprintln!("Discarded oversized COBS message");
        encoded.clear();
    }

    false
}

fn decode_and_dispatch(
    encoded: &[u8],
    decoded: &mut [u8],
    output_directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let report = cobs::decode(encoded, decoded)?;
    handlers::dispatch_message(&decoded[..report.frame_size()], output_directory)
}
