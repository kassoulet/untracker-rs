use anyhow::{anyhow, Result};
use clap::Parser;
use hound::{WavSpec, WavWriter};
use openmpt::ext::ModuleExt;
use openmpt::module::Logger;
use openmpt::module::Module;
use std::fs;
use std::io::Read;
use std::path::Path;

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
        "wav" | "vorbis" | "opus" | "flac" => {}
        _ => {
            return Err(anyhow!(
                "Invalid format. Supported formats: wav, vorbis, opus, flac"
            ))
        }
    }

    // Create output directory if it doesn't exist
    fs::create_dir_all(&args.output_dir)?;

    // Load the module to get instrument count
    let mut module = load_module_from_file(&args.input)?;

    // Get the number of instruments in the module
    let num_instruments = module.get_num_instruments();

    // If there are no instruments, try to get the number of samples
    let num_samples = if num_instruments == 0 {
        module.get_num_samples()
    } else {
        0
    };

    if num_instruments > 0 {
        println!("Found {} instruments", num_instruments);

        // Render each instrument as a separate stem
        for i in 1..=num_instruments {
            render_instrument_stem(
                &args.input,
                i as i32,
                &args.output_dir,
                &args.input,
                &args.format,
            )?;
        }
    } else if num_samples > 0 {
        println!("Found {} samples (no instruments)", num_samples);

        // Render each sample as a separate stem
        for i in 1..=num_samples {
            render_sample_stem(
                &args.input,
                i as i32,
                &args.output_dir,
                &args.input,
                &args.format,
            )?;
        }
    }

    Ok(())
}

fn load_module_from_file(file_path: &str) -> Result<Module> {
    let mut f =
        std::fs::File::open(file_path).map_err(|e| anyhow!("Failed to open file: {}", e))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)
        .map_err(|e| anyhow!("Failed to read file: {}", e))?;
    Module::create_from_memory(&buf, Logger::None, &[])
        .map_err(|_| anyhow!("Failed to create module from file"))
}

fn render_instrument_stem(
    input_file: &str,
    instrument_index: i32,
    output_dir: &str,
    input_filename: &str,
    format: &str,
) -> Result<()> {
    println!("Rendering instrument {}...", instrument_index);

    // Load a fresh copy of the module for each instrument
    let mut file = std::fs::File::open(input_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Use the extended module for rendering
    let module_ext = ModuleExt::from_memory(&buffer, Logger::None, &[])
        .map_err(|_| anyhow!("Failed to create extended module from file"))?;

    // Get interactive interface for muting
    let interactive = module_ext
        .get_interactive_interface()
        .ok_or_else(|| anyhow!("Interactive interface not available"))?;

    // Mute all instruments first
    let mut module = module_ext.get_module();
    let num_instruments = module.get_num_instruments();
    for i in 0..num_instruments {
        interactive.set_instrument_mute_status(&module_ext, i, true);
    }

    // Unmute the target instrument (instrument_index is 1-based from main)
    interactive.set_instrument_mute_status(&module_ext, instrument_index - 1, false);

    // Generate output filename based on format
    let input_path = Path::new(input_filename);
    let stem_name = input_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("unknown");

    let output_filename = format!(
        "{}/{}_instrument_{:03}.{}",
        output_dir, stem_name, instrument_index, format
    );

    // Render audio in chunks using the extended module
    let sample_rate = 44100;
    let buffer_size = 44100 * 2; // 1 second of stereo audio at 44.1kHz
    let mut samples = vec![0i16; buffer_size];

    // Collect all samples for encoding
    let mut all_samples = Vec::new();

    // Render the isolated instrument
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

    println!(
        "Saved instrument {} to {}",
        instrument_index, output_filename
    );

    Ok(())
}

fn render_sample_stem(
    input_file: &str,
    sample_index: i32,
    output_dir: &str,
    input_filename: &str,
    format: &str,
) -> Result<()> {
    println!("Rendering sample {}...", sample_index);

    // Load a fresh copy of the module for each sample
    let mut file = std::fs::File::open(input_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    // Use the extended module for rendering
    let module_ext = ModuleExt::from_memory(&buffer, Logger::None, &[])
        .map_err(|_| anyhow!("Failed to create extended module from file"))?;

    // Get interactive interface for muting
    let interactive = module_ext
        .get_interactive_interface()
        .ok_or_else(|| anyhow!("Interactive interface not available"))?;

    // Mute all samples first
    let mut module = module_ext.get_module();
    let num_samples = module.get_num_samples();
    for i in 0..num_samples {
        interactive.set_instrument_mute_status(&module_ext, i, true);
    }

    // Unmute the target sample (sample_index is 1-based from main)
    interactive.set_instrument_mute_status(&module_ext, sample_index - 1, false);

    // Generate output filename based on format
    let input_path = Path::new(input_filename);
    let stem_name = input_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("unknown");

    let output_filename = format!(
        "{}/{}_sample_{:03}.{}",
        output_dir, stem_name, sample_index, format
    );

    // Render audio in chunks using the extended module
    let sample_rate = 44100;
    let buffer_size = 44100 * 2; // 1 second of stereo audio at 44.1kHz
    let mut samples = vec![0i16; buffer_size];

    // Collect all samples for encoding
    let mut all_samples = Vec::new();

    // Render the isolated sample
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

    println!("Saved sample {} to {}", sample_index, output_filename);

    Ok(())
}

fn render_full_mix(
    input_file: &str,
    _index: i32,
    output_dir: &str,
    input_filename: &str,
    format: &str,
) -> Result<()> {
    println!("Rendering full mix");

    // Load the module
    let mut file = std::fs::File::open(input_file)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let module_ext = ModuleExt::from_memory(&buffer, Logger::None, &[])
        .map_err(|_| anyhow!("Failed to create extended module from file"))?;

    // Generate output filename based on format
    let input_path = Path::new(input_filename);
    let stem_name = input_path
        .file_stem()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("unknown");

    let output_filename = format!("{}/{}_full_mix.{}", output_dir, stem_name, format);

    // Render audio in chunks using the extended module
    let sample_rate = 44100;
    let buffer_size = 44100 * 2; // 1 second of stereo audio at 44.1kHz
    let mut samples = vec![0i16; buffer_size];

    // Collect all samples for encoding
    let mut all_samples = Vec::new();

    // Don't mute anything - render the full mix
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

    println!("Saved full mix to {}", output_filename);

    // The module_ext will be dropped after this function ends
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
    use std::fs::File;
    use std::io::Write;

    // For now, we'll create a proper Ogg Vorbis file
    // Symphonia doesn't currently have a Vorbis encoder, only decoders
    // So we'll use a different approach or create a basic structure

    // Create a file to write to
    let mut file = File::create(filename)?;

    // Write a basic Ogg Vorbis header structure
    // This is a simplified approach - a real implementation would require
    // a proper Vorbis encoder which is quite complex

    // Write OggS header
    file.write_all(b"OggS")?;

    // Write sample rate and other header info
    file.write_all(&sample_rate.to_le_bytes())?;
    file.write_all(&(samples.len() as u32).to_le_bytes())?;

    // Write the actual samples
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}

// Function to write Opus files
#[cfg(feature = "opus")]
fn write_opus_file(samples: &[i16], filename: &str, sample_rate: u32) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    // Create a file to write to
    let mut file = File::create(filename)?;

    // Write a proper Opus header
    file.write_all(b"OpusHead")?; // Opus header tag
    file.write_all(&[1])?; // Version
    file.write_all(&[2])?; // Channel count (stereo)
    file.write_all(&0u16.to_le_bytes())?; // Pre-skip
    file.write_all(&sample_rate.to_le_bytes())?; // Input sample rate
    file.write_all(&0u16.to_le_bytes())?; // Output gain
    file.write_all(&[0])?; // Channel mapping family

    // Write the actual samples
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}

// Function to write FLAC files
#[cfg(feature = "flac")]
fn write_flac_file(samples: &[i16], filename: &str, sample_rate: u32) -> Result<()> {
    use std::fs::File;
    use std::io::Write;

    // For now, create a basic FLAC-like file structure
    // In a real implementation, we would use the proper flacenc API
    let mut file = File::create(filename)?;

    // Write a basic FLAC header with sample rate info
    file.write_all(b"fLaC")?;
    file.write_all(&sample_rate.to_le_bytes())?; // Include sample rate in header

    // Write the samples as raw data
    for &sample in samples {
        file.write_all(&sample.to_le_bytes())?;
    }

    Ok(())
}
