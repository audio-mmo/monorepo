mod shared;

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use anyhow::Result;
use structopt::StructOpt;
use synthizer as syz;

use shared::*;

#[derive(StructOpt)]
struct Params {
    #[structopt(long = "--base-path")]
    base_path: String,
    #[structopt(long = "-asset")]
    asset: String,
}

fn main() -> Result<()> {
    env_logger::init();
    log::info!("Starting...");

    let params = Params::from_args();
    let _guard = syz::initialize()?;

    let io_impl = IoProviderImpl::new(Path::new(params.base_path.as_str()))?;
    let engine = ammo_sonifier::Engine::new(Box::new(io_impl))?;
    log::info!("Engine is up");

    let buffer = engine.new_buffer(params.asset)?;
    let object = engine.new_object(syz::PannerStrategy::Hrtf, (0.5, 1.0, 0.0))?;
    let player = engine.new_buffer_player(&buffer)?;
    player.set_looping(true)?;
    player.connect(&object)?;

    println!("Press ctrl+c to exit");
    loop {
        for i in (-10..9).chain((-9..=10).rev()) {
            object.set_position((i as f64 / 10.0, 1.0, 0.0))?;
            sleep(Duration::from_millis(100));
        }
    }
}
