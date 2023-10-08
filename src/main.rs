use core::panic;
use std::{
    env,
    fs::File,
    io::{self, prelude::*},
    num::NonZeroU64,
};

use clap::Parser;
use log::{info, LevelFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None,arg_required_else_help = true)]
struct Args {
    ///Verbose mode
    #[arg(short, long, default_value = "false")]
    verbose: bool,

    ///Gzip Output to gzip format.
    ///Zlib Output to deflate format instead of gzip.
    ///Deflat Output to deflate format instead of gzip.
    #[arg(short, long, default_value = "Gzip", verbatim_doc_comment)]
    outputtype: String,

    ///Perform # iterations, more gives more compression but is slower.
    #[arg(short, long, default_value = "15")]
    iterations: u64,

    ///Input files, at least one file need to be specified
    #[clap(short, long, required = true)]
    filename: String,
}

fn main() {
    let args = Args::parse();
    let mut options = zopfli::Options::default();

    let output_type = match args.outputtype.try_into() {
        Ok(outputtype) => outputtype,
        Err(e) => panic!("Error : {e}"),
    };

    let extension = match output_type {
        zopfli::Format::Gzip => ".gz",
        zopfli::Format::Zlib => ".zlib",
        zopfli::Format::Deflate => ".deflate",
    };

    // Determine the log level based on command-line argument or RUST_LOG
    let log_level = if args.verbose {
        LevelFilter::Trace
    } else {
        // If the --verbose flag is not set, use the RUST_LOG environment variable.
        // If RUST_LOG is not set, use the default level (e.g., LevelFilter::Off).
        env::var("RUST_LOG")
            .ok()
            .and_then(|level| level.parse().ok())
            .unwrap_or(LevelFilter::Off)
    };
    env_logger::builder().filter(None, log_level).init();

    options.iteration_count = match args.iterations {
        0 => panic!("Error: must have 1 or more iterations"),
        _ => NonZeroU64::new(args.iterations).unwrap(),
    };

    // TODO: Allow specifying output to STDOUT

    let filename = args.filename;
    let file =
        File::open(&filename).unwrap_or_else(|why| panic!("couldn't open {}: {}", filename, why));
    let filesize = file.metadata().map(|x| x.len()).unwrap() as usize;

    let out_filename = format!("{}{}", filename, extension);

    // Attempt to create the output file, panic if the output file could not be opened
    let out_file = File::create(&out_filename)
        .unwrap_or_else(|why| panic!("couldn't create output file {}: {}", out_filename, why));
    let mut out_file = WriteStatistics::new(out_file);

    zopfli::compress(options, output_type, &file, &mut out_file)
        .unwrap_or_else(|why| panic!("couldn't write to output file {}: {}", out_filename, why));

    let out_size = out_file.count;
    info!(
        "Original Size: {}, Compressed: {}, Compression: {}% Removed",
        filesize,
        out_size,
        100.0 * (filesize - out_size) as f64 / filesize as f64
    );
}

struct WriteStatistics<W> {
    inner: W,
    count: usize,
}

impl<W> WriteStatistics<W> {
    fn new(inner: W) -> Self {
        WriteStatistics { inner, count: 0 }
    }
}

impl<W: Write> Write for WriteStatistics<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let res = self.inner.write(buf);
        if let Ok(size) = res {
            self.count += size;
        }
        res
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
