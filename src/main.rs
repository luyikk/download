use anyhow::Result;
use download_lib::DownloadFile;
use log::LevelFilter;
use std::path::PathBuf;
use std::time::Duration;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    let opt = Opt::from_args();

    env_logger::builder()
        .filter_module("want", LevelFilter::Error)
        .filter_module("mio", LevelFilter::Error)
        .filter_module("rustls", LevelFilter::Error)
        .filter_level(LevelFilter::Trace)
        .init();

    match DownloadFile::start_download(opt.url, opt.save_path, opt.tasks, 1024 * 1024, opt.name)
        .await
    {
        Ok(download) => {
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
                let percent = status.get_percent_complete();
                let speed_kb = status.get_byte_sec() / 1024;
                if percent > 0.0 {
                    log::info!("speed of progress:{}% {} K/s", percent, speed_kb);
                } else {
                    // Unknown size - show downloaded amount instead
                    let down_mb = status.get_down_size() as f64 / 1024.0 / 1024.0;
                    log::info!("downloaded: {:.2} MB  {} K/s", down_mb, speed_kb);
                }
            }

            if !status.is_error() {
                log::info!(
                    "url {} download finish,save to {}",
                    status.url(),
                    download.get_real_file_path()
                );
            } else {
                log::info!(
                    "url {} download is error:{}",
                    status.url(),
                    status.get_error().unwrap()
                );
            }
        }
        Err(err) => {
            log::error!("down file fail:{}", err);
        }
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

    /// custom filename for the downloaded file
    #[structopt(short = "n", long)]
    name: Option<String>,
}
