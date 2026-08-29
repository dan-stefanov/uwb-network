use std::fs::File;
use std::io;
use std::path::Path;

use chrono::Utc;
use npyz::WriterBuilder;
use num_complex::Complex;

const CIR_MAGIC: u64 = 0xd2d8_49a7_1e10_c9a1;
const MAGIC_LENGTH: usize = size_of::<u64>();
const MESSAGE_TYPE_LENGTH: usize = size_of::<u32>();
const MESSAGE_HEADER_LENGTH: usize = MAGIC_LENGTH + MESSAGE_TYPE_LENGTH;
const CIR_SAMPLE_LENGTH: usize = 2 * size_of::<i32>();
const MAX_CIR_LENGTH: usize = 1016;

#[repr(u32)]
enum MessageType {
    Cir = 1,
}

pub fn dispatch_message(
    message: &[u8],
    output_directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if message.len() < MESSAGE_HEADER_LENGTH {
        return Err("message is shorter than its header".into());
    }

    let magic = u64::from_le_bytes(message[..MAGIC_LENGTH].try_into()?);
    if magic != CIR_MAGIC {
        return Err(format!("unexpected message magic: {magic:#018x}").into());
    }

    let message_type = u32::from_le_bytes(message[MAGIC_LENGTH..MESSAGE_HEADER_LENGTH].try_into()?);
    match message_type {
        message_type if message_type == MessageType::Cir as u32 => {
            let samples = deserialize_cir(&message[MESSAGE_HEADER_LENGTH..])?;
            store_cir(&samples, output_directory)
        }
        _ => Err(format!("unsupported message type: {message_type}").into()),
    }
}

fn deserialize_cir(payload: &[u8]) -> Result<Vec<Complex<i32>>, Box<dyn std::error::Error>> {
    if payload.len() < size_of::<u16>() {
        return Err("CIR message is shorter than its length".into());
    }

    let cir_length = u16::from_le_bytes(payload[..size_of::<u16>()].try_into()?);
    let cir_length = usize::from(cir_length);
    let encoded_samples = &payload[size_of::<u16>()..];
    let expected_length = cir_length * CIR_SAMPLE_LENGTH;
    if cir_length > MAX_CIR_LENGTH || encoded_samples.len() != expected_length {
        return Err(format!(
            "invalid CIR message length: {cir_length} samples, {} bytes",
            encoded_samples.len()
        )
        .into());
    }

    let (encoded_samples, []) = encoded_samples.as_chunks::<CIR_SAMPLE_LENGTH>() else {
        unreachable!("CIR sample payload length was validated above");
    };
    let samples = encoded_samples
        .iter()
        .map(|sample| {
            let real = i32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
            let imaginary = i32::from_le_bytes([sample[4], sample[5], sample[6], sample[7]]);
            Complex::new(real, imaginary)
        })
        .collect();

    Ok(samples)
}

fn store_cir(
    samples: &[Complex<i32>],
    output_directory: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = Utc::now().format("%Y-%m-%dT%H-%M-%S%.3fZ");
    let output_path = output_directory.join(format!("{timestamp}-cir.npy"));
    let output_file = match File::options()
        .write(true)
        .create_new(true)
        .open(&output_path)
    {
        Ok(output_file) => output_file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            println!("Skipped existing CIR capture at {}", output_path.display());
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let mut writer = npyz::WriteOptions::new()
        .default_dtype()
        .shape(&[samples.len() as u64, 2])
        .writer(output_file)
        .begin_nd()?;
    for sample in samples {
        writer.push(&sample.re)?;
        writer.push(&sample.im)?;
    }
    writer.finish()?;

    println!(
        "Saved {} CIR samples to {}",
        samples.len(),
        output_path.display()
    );
    Ok(())
}
