//! Set up a buffer and some music.
mod shared;

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use rand::prelude::*;
use structopt::StructOpt;
use synthizer as syz;

use shared::*;

#[derive(StructOpt)]
struct Params {
    #[structopt(long = "--base-path")]
    base_path: String,
    #[structopt(long = "--direct")]
    direct: String,
    #[structopt(long = "--panned")]
    panned: String,
}

fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting...");

    let params = Params::from_args();
    let _guard = syz::initialize()?;

    let io_impl = IoProviderImpl::new(Path::new(params.base_path.as_str()))?;
    let engine = ammo_sonifier::Engine::new(Box::new(io_impl))?;
    log::info!("Engine is up");

    let direct_buffer = engine.new_buffer(params.direct)?;
    let panned_buffer = engine.new_buffer(params.panned)?;

    loop {
        engine.ui_sound_direct(&direct_buffer, thread_rng().gen())?;
        sleep(Duration::from_millis(1000));

        engine.ui_sound_panned(
            &panned_buffer,
            thread_rng().gen(),
            thread_rng().gen_range(-1.0..=1.0),
        )?;
        sleep(Duration::from_millis(1000));
        break;
    }

    sleep(Duration::from_millis(5000));
    Ok(())
}
