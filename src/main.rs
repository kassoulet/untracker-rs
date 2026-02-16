use anyhow::{anyhow, Result};
use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use openmpt::ext::ModuleExt;
use openmpt::module::Logger;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use untracker::{render_stem, AudioFormat, ExportOptions, ResampleMethod};

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

    /// Render stems in parallel
    #[arg(short, long)]
    parallel: bool,
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
    // Initialize logging with a nice format, but only in non-test mode
    if !cfg!(test) {
        env_logger::Builder::from_default_env()
            .format(|buf, record| {
                writeln!(
                    buf,
                    "{} [{}] - {}",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                    record.level(),
                    record.args()
                )
            })
            .init();
    }

    let args = Args::parse();
    let format: AudioFormat = args.format.parse()?;

    if args.channels != 1 && args.channels != 2 {
        return Err(anyhow!(
            "Only 1 (mono) or 2 (stereo) channels are supported"
        ));
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

    info!("Loading module file: {}", args.input);

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

    let indices: Vec<i32> = (0..(if num_instruments > 0 {
        num_instruments
    } else {
        num_samples
    }))
        .collect();
    let is_instrument = num_instruments > 0;

    let total_stems = indices.len();
    info!(
        "Found {} {} to extract",
        total_stems,
        if is_instrument {
            "instruments"
        } else {
            "samples"
        }
    );

    // Create progress bar
    let pb = ProgressBar::new(total_stems as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) - {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );

    // Show initial summary message
    if is_instrument {
        println!("Extracting {} instrument stems", num_instruments);
    } else {
        println!("Extracting {} sample stems", num_samples);
    }

    if !cfg!(test) {
        if is_instrument {
            pb.set_message(format!("Extracting {} instrument stems", num_instruments));
        } else {
            pb.set_message(format!("Extracting {} sample stems", num_samples));
        }
    }

    if args.parallel {
        use rayon::prelude::*;

        if cfg!(test) {
            // For tests, run without progress bar
            indices.into_par_iter().try_for_each(|i| {
                render_stem(
                    &buffer,
                    i,
                    is_instrument,
                    &args.output_dir,
                    stem_name,
                    &options,
                    None,
                )
            })?;
        } else {
            use indicatif::ParallelProgressIterator;
            // For normal execution, use progress bar
            indices
                .into_par_iter()
                .progress_with(pb.clone())
                .try_for_each(|i| {
                    render_stem(
                        &buffer,
                        i,
                        is_instrument,
                        &args.output_dir,
                        stem_name,
                        &options,
                        Some(&pb),
                    )
                })?;
        }
    } else {
        for i in indices {
            render_stem(
                &buffer,
                i,
                is_instrument,
                &args.output_dir,
                stem_name,
                &options,
                if cfg!(test) { None } else { Some(&pb) },
            )?;
            if !cfg!(test) {
                pb.inc(1);
            }
        }
    }

    if !cfg!(test) {
        pb.finish_with_message(format!("Completed extracting {} stems!", total_stems));
    }
    println!("Completed extracting {} stems!", total_stems);
    Ok(())
}

fn read_file_to_buffer(path: &str) -> Result<Vec<u8>> {
    log::info!("Reading input file: {}", path);
    let mut file = fs::File::open(path)?;
    let mut buffer = Vec::new();
    let bytes_read = file.read_to_end(&mut buffer)?;
    log::info!("Successfully read {} bytes from {}", bytes_read, path);
    Ok(buffer)
}
