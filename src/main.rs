use anyhow::{Result, anyhow};
use clap::Parser;
use hound::{WavSpec, WavWriter};
use openmpt::module::Module;
use openmpt::ext::{ModuleExt};
use openmpt::module::Logger;
use std::fs;
use std::io::Read;
use std::path::Path;

#[cfg(feature = "vorbis")]
use std::fs::File;

#[cfg(feature = "flac")]
use std::fs::File as FlacFile;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input module file path
    #[arg(short, long)]
    input: String,

    /// Output directory for stem files
    #[arg(short, long, default_value = ".")]
    output_dir: String,
    
    /// Output format: wav, vorbis, opus, flac
    #[arg(short, long, default_value = "wav")]
    format: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Validate format
    match args.format.as_str() {
        "wav" | "vorbis" | "opus" | "flac" => {},
        _ => return Err(anyhow!("Invalid format. Supported formats: wav, vorbis, opus, flac")),
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(&args.output_dir)?;

    // Load the module to get instrument count
    let mut module = load_module_from_file(&args.input)?;

    // Get the number of instruments in the module
    let num_instruments = module.get_num_instruments();

    println!("Found {} instruments", num_instruments);

    // Render each instrument as a separate stem
    for i in 1..=num_instruments {
        render_instrument_stem(&args.input, i as i32, &args.output_dir, &args.input, &args.format)?;
    }

    Ok(())
}

fn load_module_from_file(file_path: &str) -> Result<Module> {
    let mut f = std::fs::File::open(file_path).map_err(|e| anyhow!("Failed to open file: {}", e))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| anyhow!("Failed to read file: {}", e))?;
    Module::create_from_memory(&buf, Logger::None, &[]).map_err(|_| anyhow!("Failed to create module from file"))
}

fn render_instrument_stem(
    input_file: &str,
    instrument_index: i32,
    output_dir: &str,
    input_filename: &str,
    format: &str,
) -> Result<()> {
    println!("Rendering instrument {}", instrument_index);

    // Load a fresh copy of the module for each instrument
    let mut file = std::fs::File::open(input_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let module_ext = ModuleExt::from_memory(&buffer, Logger::None, &[]).map_err(|_| anyhow!("Failed to create extended module from file"))?;
    let interactive_interface = module_ext.get_interactive_interface().ok_or_else(|| anyhow!("Failed to get interactive interface"))?;

    // Mute all instruments first
    let num_instruments = module_ext.get_module().get_num_instruments();
    for inst in 1..=num_instruments {
        interactive_interface.set_instrument_mute_status(&module_ext, inst, true);
    }

    // Unmute only the current instrument
    interactive_interface.set_instrument_mute_status(&module_ext, instrument_index, false);

    // Generate output filename based on format
    let input_path = Path::new(input_filename);
    let stem_name = input_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("unknown");

    let output_filename = format!(
        "{}/{}_stem_{}.{}",
        output_dir, stem_name, instrument_index, format
    );

    // Render audio in chunks using the extended module
    let sample_rate = 44100;
    let buffer_size = 44100 * 2; // 1 second of stereo audio at 44.1kHz
    let mut samples = vec![0i16; buffer_size];

    // Collect all samples for encoding
    let mut all_samples = Vec::new();
    loop {
        let frames_rendered = module_ext.read_interleaved_stereo(sample_rate, &mut samples);

        if frames_rendered == 0 {
            break; // No more audio to render
        }

        // Add the rendered samples to our collection
        all_samples.extend_from_slice(&samples[..frames_rendered * 2]);

        // Check if we've reached the end of the song
        if module_ext.get_position_seconds() >= module_ext.get_duration_seconds() {
            break;
        }
    }

    // Write the audio data based on the selected format
    match format {
        "wav" => write_wav_file(&all_samples, &output_filename, sample_rate as u32)?,
        #[cfg(feature = "vorbis")]
        "vorbis" => write_vorbis_file(&all_samples, &output_filename, sample_rate as u32)?,
        #[cfg(feature = "opus")]
        "opus" => write_opus_file(&all_samples, &output_filename, sample_rate as u32)?,
        #[cfg(feature = "flac")]
        "flac" => write_flac_file(&all_samples, &output_filename, sample_rate as u32)?,
        _ => return Err(anyhow!("Unsupported format: {}", format)),
    }

    println!("Saved instrument {} to {}", instrument_index, output_filename);

    // The module_ext and interactive_interface will be dropped after this function ends
    Ok(())
}

// Function to write WAV files
fn write_wav_file(samples: &[i16], filename: &str, sample_rate: u32) -> Result<()> {
    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(filename, spec)?;

    for chunk in samples.chunks_exact(2) {
        writer.write_sample(chunk[0])?;
        writer.write_sample(chunk[1])?;
    }

    writer.finalize()?;
    Ok(())
}

// Function to write Vorbis files
#[cfg(feature = "vorbis")]
fn write_vorbis_file(samples: &[i16], filename: &str, sample_rate: u32) -> Result<()> {
    // Placeholder implementation - in a real implementation, we would use a proper Vorbis encoder
    // For now, we'll just write a simple OGG Vorbis header followed by the samples
    use std::io::Write;
    
    // This is a simplified placeholder - a real implementation would require proper Vorbis encoding
    let mut file = File::create(filename)?;
    
    // Write a simple header indicating this is a placeholder file
    writeln!(file, "VORBIS_PLACEHOLDER_FILE")?;
    writeln!(file, "Sample Rate: {}", sample_rate)?;
    writeln!(file, "Samples: {}", samples.len())?;
    
    // In a real implementation, we would encode the samples with proper Vorbis encoding
    Ok(())
}

// Function to write Opus files
#[cfg(feature = "opus")]
fn write_opus_file(samples: &[i16], filename: &str, sample_rate: u32) -> Result<()> {
    // Placeholder implementation for Opus encoding
    // The opus crate is quite low-level, so we'll create a basic implementation
    std::fs::write(filename, "Opus file placeholder")?;
    Ok(())
}

// Function to write FLAC files
#[cfg(feature = "flac")]
fn write_flac_file(samples: &[i16], filename: &str, sample_rate: u32) -> Result<()> {
    use std::io::Write;
    
    // Placeholder implementation - in a real implementation, we would use a proper FLAC encoder
    let mut file = FlacFile::create(filename)?;
    
    // Write a simple header indicating this is a placeholder file
    writeln!(file, "FLAC_PLACEHOLDER_FILE")?;
    writeln!(file, "Sample Rate: {}", sample_rate)?;
    writeln!(file, "Samples: {}", samples.len())?;
    
    // In a real implementation, we would encode the samples with proper FLAC encoding
    Ok(())
}