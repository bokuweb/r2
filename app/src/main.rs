use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use anyhow::{bail, Result};

use core::{
    bus::{Bus, RAM_START},
    clint::Clint,
    start,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    /// Path to image file.
    image_file_path: PathBuf,

    #[arg(short, long)]
    /// Path to dtb file.
    dtb_file_path: Option<PathBuf>,

    #[arg(short, long, default_value = "67108864")]
    /// RAM size. default 64 * 1024 * 1024.
    ram_size: usize,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let ram_size = args.ram_size;

    let mut ram = vec![0u8; ram_size];

    let mut f = File::open(args.image_file_path)?;

    let len = f.metadata()?.len();
    if len > ram_size as u64 {
        bail!("Insufficient RAM capacity. Please increase RAM capacity with `-r` option.")
    }

    f.read_exact(&mut ram[..len as usize])?;

    let dtb_ref = if let Some(dtb) = args.dtb_file_path {
        let mut f = File::open(dtb)?;
        let len = f.metadata()?.len();
        let ptr = ram_size as u64 - len;
        f.read_exact(&mut ram[(ptr as usize)..(ptr + len) as usize])?;
        ptr as u32 + RAM_START
    } else {
        0
    };

    let clint = Clint::new(devices::timer::Timer::default());
    let uart = devices::uart::Uart::new();
    let bus = Bus::new(ram, clint, uart);

    start(bus, RAM_START, dtb_ref, &std::thread::sleep);

    Ok(())
}
