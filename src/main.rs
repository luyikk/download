#![feature(backtrace)]
use anyhow::Result;
use download_lib::DownloadFile;
use log::LevelFilter;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    env_logger::builder()
        .filter_module("want", LevelFilter::Error)
        .filter_module("mio", LevelFilter::Error)
        .filter_level(LevelFilter::Trace)
        .init();

    let download = DownloadFile::start_download(opt.url, opt.save_path, opt.tasks).await?;

    let status = download.get_status();
    //  tokio::spawn(async move{
    //      while !status.is_finish() {
    //          tokio::time::sleep(Duration::from_secs(1)).await;
    //          log::info!("speed of progress:{}% {} K/s",status.get_percent_complete(),status.get_byte_sec()/1024);
    //      }
    //  });
    //
    //  while !download.is_finish() {
    //      let mut s="".to_string();
    //      std::io::stdin().read_line(&mut s).unwrap();
    //      if download.is_start() {
    //          download.suspend()
    //      }else{
    //          download.restart();
    //      }
    //  }

    while !status.is_finish() {
        tokio::time::sleep(Duration::from_secs(1)).await;
        log::info!(
            "speed of progress:{}% {} K/s",
            status.get_percent_complete(),
            status.get_byte_sec() / 1024
        );
    }

    if !status.is_error() {
        log::info!(
            "url {} download finish,save to {}",
            status.url(),
            download.get_real_file_path()
        );
    } else {
        log::info!("url {} download is error", status.url());
    }
    Ok(())
}

// A basic example
#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    /// http url,http server need support range
    #[structopt(short = "u", long)]
    url: String,

    /// save file path
    #[structopt(short = "s", long, parse(from_os_str), default_value = "./")]
    save_path: PathBuf,

    /// number of concurrent download
    #[structopt(short = "t", long, default_value = "15")]
    tasks: u64,
}
