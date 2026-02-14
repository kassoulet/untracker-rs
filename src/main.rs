mod audio;

use anyhow::{anyhow, Result};
use audio::{AudioFormat, ExportOptions, ResampleMethod, write_audio_file};
use clap::{Parser, ValueEnum};
use openmpt::ext::ModuleExt;
use openmpt::module::Logger;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
/// Untracker: Stem extractor for tracker modules (MOD, S3M, XM, IT, etc.)
struct Args {
    /// Input module file path
    #[arg(short, long)]
    input: String,

    /// Output directory for stem files
    #[arg(short, long)]
    output_dir: String,

    /// Sample rate
    #[arg(long, default_value_t = 44100)]
    sample_rate: u32,

    /// Number of channels (1 or 2)
    #[arg(long, default_value_t = 2)]
    channels: u32,

    /// Resampling method
    #[arg(long, default_value = "sinc")]
    resample: ResampleMethodArg,

    /// Output format: wav, vorbis, opus, flac
    #[arg(long, default_value = "wav")]
    format: String,

    /// Bit depth for lossless formats (16 or 24)
    #[arg(long, default_value_t = 16)]
    bit_depth: u32,

    /// Bitrate for Opus format in kbps
    #[arg(long, default_value_t = 128)]
    opus_bitrate: u32,

    /// Vorbis quality level (0-10)
    #[arg(long, default_value_t = 5)]
    vorbis_quality: u32,

    /// Stereo separation in percent (0-200)
    #[arg(long, default_value_t = 100)]
    stereo_separation: u32,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum ResampleMethodArg {
    Nearest,
    Linear,
    Cubic,
    Sinc,
}

impl From<ResampleMethodArg> for ResampleMethod {
    fn from(arg: ResampleMethodArg) -> Self {
        match arg {
            ResampleMethodArg::Nearest => ResampleMethod::Nearest,
            ResampleMethodArg::Linear => ResampleMethod::Linear,
            ResampleMethodArg::Cubic => ResampleMethod::Cubic,
            ResampleMethodArg::Sinc => ResampleMethod::Sinc,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let format: AudioFormat = args.format.parse()?;

    if args.channels != 1 && args.channels != 2 {
        return Err(anyhow!("Only 1 (mono) or 2 (stereo) channels are supported"));
    }

    if args.bit_depth != 16 && args.bit_depth != 24 {
        return Err(anyhow!("Only 16 or 24 bit depth is supported"));
    }

    let options = ExportOptions {
        format,
        sample_rate: args.sample_rate,
        channels: args.channels,
        bit_depth: args.bit_depth,
        opus_bitrate: args.opus_bitrate,
        vorbis_quality: args.vorbis_quality,
        resample: args.resample.into(),
        stereo_separation: args.stereo_separation as i32,
    };

    fs::create_dir_all(&args.output_dir)?;

    let buffer = read_file_to_buffer(&args.input)?;
    let module_ext = ModuleExt::from_memory(&buffer, Logger::None, &[])
        .map_err(|_| anyhow!("Failed to load module"))?;

    let mut module = module_ext.get_module();
    let num_instruments = module.get_num_instruments();
    let num_samples = module.get_num_samples();

    let stem_name = Path::new(&args.input)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("stem");

    if num_instruments > 0 {
        println!("Extracting {} instrument stems...", num_instruments);
        for i in 0..num_instruments {
            render_stem(&buffer, i, true, &args.output_dir, stem_name, &options)?;
        }
    } else {
        println!("Extracting {} sample stems (no instruments found)...", num_samples);
        for i in 0..num_samples {
            render_stem(&buffer, i, false, &args.output_dir, stem_name, &options)?;
        }
    }

    Ok(())
}

fn read_file_to_buffer(path: &str) -> Result<Vec<u8>> {
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn render_stem(
    buffer: &[u8],
    index: i32,
    is_instrument: bool,
    output_dir: &str,
    base_name: &str,
    options: &ExportOptions,
) -> Result<()> {
    let type_label = if is_instrument { "instrument" } else { "sample" };
    println!("  Rendering {} {}...", type_label, index + 1);

    let module_ext = ModuleExt::from_memory(buffer, Logger::None, &[])
        .map_err(|_| anyhow!("Failed to re-load module for rendering"))?;

    let interactive = module_ext
        .get_interactive_interface()
        .ok_or_else(|| anyhow!("Interactive interface not available"))?;

    let mut module = module_ext.get_module();
    
    // Configure render parameters
    module.set_render_interpolation_filter_length(options.resample.to_openmpt_filter_length());
    module.set_render_stereo_separation(options.stereo_separation);

    let count = if is_instrument {
        module.get_num_instruments()
    } else {
        module.get_num_samples()
    };

    // Mute everything except the target
    for i in 0..count {
        interactive.set_instrument_mute_status(&module_ext, i, i != index);
    }

    let ext_str = match options.format {
        AudioFormat::Wav => "wav",
        #[cfg(feature = "vorbis")]
        AudioFormat::Vorbis => "ogg",
        #[cfg(feature = "opus")]
        AudioFormat::Opus => "opus",
        #[cfg(feature = "flac")]
        AudioFormat::Flac => "flac",
    };

    let output_path = format!(
        "{}/{}_{}_{:03}.{}",
        output_dir, base_name, type_label, index + 1, ext_str
    );

    let mut samples = vec![0i16; 8192];
    let mut all_audio = Vec::new();

    loop {
        let rendered = if options.channels == 2 {
            module_ext.read_interleaved_stereo(options.sample_rate as i32, &mut samples)
        } else {
            module.read_mono(options.sample_rate as i32, &mut samples[..4096])
        };

        if rendered == 0 { break; }
        
        let num_samples_to_copy = rendered * (options.channels as usize);
        all_audio.extend_from_slice(&samples[..num_samples_to_copy]);
        
        if module_ext.get_position_seconds() >= module_ext.get_duration_seconds() {
            break;
        }
    }

    write_audio_file(&all_audio, &output_path, options)?;
    Ok(())
}
