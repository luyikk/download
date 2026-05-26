use anyhow::Result;
use download_lib::DownloadFile;
use humansize::{format_size, BINARY};
use indicatif::{ProgressBar, ProgressStyle};
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
        .filter_module("download_lib", LevelFilter::Error)
        .filter_level(LevelFilter::Info)
        .init();

    match DownloadFile::start_download(
        opt.url,
        opt.save_path,
        opt.tasks,
        1024 * 1024,
        opt.name,
        opt.cookies,
    )
    .await
    {
        Ok(download) => {
            let status = download.get_status();
            let total = status.get_size();
            let size_known = total > 0;

            // Build the correct progress bar style based on whether we know the size
            let pb = if size_known {
                let pb = ProgressBar::new(total);
                pb.set_style(
                    ProgressStyle::with_template(
                        "{spinner:.green} [{elapsed_precise}] [{bar:45.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, ETA {eta})",
                    )
                    .unwrap()
                    .progress_chars("█▉▊▋▌▍▎▏  "),
                );
                pb
            } else {
                let pb = ProgressBar::new_spinner();
                pb.set_style(
                    ProgressStyle::with_template(
                        "{spinner:.green} [{elapsed_precise}] {bytes} downloaded ({bytes_per_sec})",
                    )
                    .unwrap(),
                );
                pb
            };

            pb.enable_steady_tick(Duration::from_millis(120));

            while !status.is_finish() {
                tokio::time::sleep(Duration::from_millis(200)).await;
                let down = status.get_down_size();
                pb.set_position(down);

                // Update the speed suffix manually for spinner mode
                if !size_known {
                    let speed = status.get_byte_sec();
                    pb.set_message(format!("{}/s", format_size(speed, BINARY)));
                }
            }

            // Finish the bar cleanly
            if !status.is_error() {
                pb.set_position(if size_known {
                    total
                } else {
                    status.get_down_size()
                });
                pb.finish_with_message("✓ done");
                log::info!("saved to: {}", download.get_real_file_path());
            } else {
                pb.abandon_with_message("✗ failed");
                log::error!("download error: {}", status.get_error().unwrap());
            }
        }
        Err(err) => {
            log::error!("down file fail: {}", err);
        }
    }

    Ok(())
}

#[derive(StructOpt, Debug)]
#[structopt(name = "durl", about = "Fast multi-task HTTP downloader")]
struct Opt {
    /// HTTP URL to download
    #[structopt(short = "u", long)]
    url: String,

    /// Save file path (directory or full path)
    #[structopt(short = "s", long, parse(from_os_str), default_value = "./")]
    save_path: PathBuf,

    /// Number of concurrent download tasks
    #[structopt(short = "t", long, default_value = "15")]
    tasks: u64,

    /// Custom output filename
    #[structopt(short = "n", long)]
    name: Option<String>,

    /// Cookies in JSON format, e.g. '{"session":"abc"}' or '[{"name":"s","value":"abc"}]'
    #[structopt(short = "c", long)]
    cookies: Option<String>,
}
