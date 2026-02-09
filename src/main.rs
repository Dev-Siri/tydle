use std::process;

use anyhow::{Result, anyhow};
use clap::Parser;
use colored::Colorize;
use tokio::fs;
use tydle::{
    Cipher, DiskCacheStore, Ext, Extract, Filterable, Tydle, TydleOptions, VideoId, YtStream,
    YtStreamSource, cookies::parse_netscape_cookies,
};

use crate::{
    format::{Format, compact_num, get_resolution, human_readable_size, parse_format},
    stream_downloader::StreamDownloader,
};

mod format;
mod stream_downloader;

#[derive(Parser, Debug)]
#[clap(version)]
struct TydleArgs {
    /// Client-side IP address to bind to.
    #[arg(long)]
    source_ip: Option<String>,
    /// Netscape formatted file to read cookies from and dump cookie jar in.
    #[arg(long)]
    cookies: Option<String>,
    /// Use an unencrypted connection to retrieve information about the video.
    #[arg(long)]
    prefer_insecure: bool,
    #[arg(long)]
    /// List available formats of each video.
    list_formats: bool,
    #[arg(long)]
    get_url: bool,
    /// Specify the type of format to download the stream of.
    #[arg(long, short)]
    format: Option<String>,
    // Where to output the final downloaded stream.
    #[arg(long)]
    out: Option<String>,
    video_id: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run().await {
        log::error!("{}", e.to_string());
        process::exit(1);
    }

    Ok(())
}

async fn run() -> Result<()> {
    let args = TydleArgs::parse();
    let auth_cookies = match args.cookies {
        Some(cookies_path) => {
            let cookie_file_content = fs::read_to_string(cookies_path).await?;
            parse_netscape_cookies(cookie_file_content)?
        }
        None => Default::default(),
    };

    let cache_path = std::env::temp_dir().join("tydle_cache");
    let format = parse_format(args.format.unwrap_or("bestvideo".into()).as_str())?;

    tydle::logger::init_logging("info");
    let tydle = Tydle::new_with_cache(
        TydleOptions {
            auth_cookies,
            prefer_insecure: args.prefer_insecure,
            source_address: args.source_ip.unwrap_or_default(),
            ..Default::default()
        },
        DiskCacheStore::new(cache_path.clone()),
        DiskCacheStore::new(cache_path),
    )?;

    let video_id = VideoId::new(args.video_id)?;
    let yt_stream_response = tydle.get_streams(&video_id).await?;

    log::info!("Got player URL: {}", yt_stream_response.player_url);

    if args.list_formats {
        list_formats(&yt_stream_response.streams);
    }

    let download_stream = match format {
        Format::BestAudio => {
            let mut streams = yt_stream_response
                .streams
                .audio_only()
                .with_highest_bitrate()
                .into_iter()
                .collect::<Vec<_>>();

            streams.sort_by_key(|s| match s.ext {
                Ext::M4a | Ext::Mp4 => 0,
                _ => 1,
            });

            streams
                .first()
                .cloned()
                .ok_or(anyhow!("No matching stream."))
        }
        Format::BestVideo => {
            let mut streams = yt_stream_response
                .streams
                .video_only()
                .with_highest_bitrate()
                .into_iter()
                .collect::<Vec<_>>();

            streams.sort_by_key(|s| match s.ext {
                Ext::M4a | Ext::Mp4 => 0,
                _ => 1,
            });

            streams
                .first()
                .cloned()
                .ok_or(anyhow!("No matching stream."))
        }
        Format::WorstAudio => {
            let streams = yt_stream_response
                .streams
                .audio_only()
                .with_lowest_bitrate();
            streams
                .into_iter()
                .collect::<Vec<_>>()
                .first()
                .cloned()
                .ok_or(anyhow!("No matching stream."))
        }
        Format::WorstVideo => {
            let streams = yt_stream_response
                .streams
                .video_only()
                .with_lowest_bitrate();
            streams
                .into_iter()
                .collect::<Vec<_>>()
                .first()
                .cloned()
                .ok_or(anyhow!("No matching stream."))
        }
    }?;

    let output = args.out.unwrap_or(format!(
        "{}.{}",
        video_id.as_str(),
        download_stream.ext.as_str()
    ));
    let source = match download_stream.source {
        YtStreamSource::URL(url) => url,
        YtStreamSource::Signature(signature) => {
            tydle
                .decipher_signature(signature, yt_stream_response.player_url)
                .await?
        }
    };

    if !args.get_url {
        let worker_count = num_cpus::get();
        let downloader = StreamDownloader::new(worker_count);

        downloader.download(&source, &output).await?;
    } else {
        println!("{}", source);
    }

    Ok(())
}

fn list_formats(streams: &Vec<YtStream>) {
    println!(
        "{:<5} {:<8} {:<10} {:<3} | {:<12} {:<10} {:<6} | {:<14} {}",
        "ID".yellow(),
        "EXT".yellow(),
        "RESOLUTION".yellow(),
        "FPS".yellow(),
        "FILESIZE".yellow(),
        "TBR".yellow(),
        "PROTO".yellow(),
        "VCODEC".yellow(),
        "ACODEC".yellow(),
    );
    println!("{}", "-".repeat(100));

    for stream in streams {
        let resolution = get_resolution(stream.height, stream.width);
        println!(
            "{:<5} {:<8} {:<10} {:<3} | {:<12} {:<10} {:<6} | {:<14} {}",
            stream.itag.to_string().green(),
            stream.ext.as_str(),
            if resolution == "" {
                "audio only"
            } else {
                resolution.as_str()
            },
            stream.fps,
            if let Some(file_size) = stream.file_size {
                format!("~{}", human_readable_size(file_size))
            } else {
                "".into()
            }
            .bright_black(),
            compact_num(stream.tbr as u64),
            "https", // stream.proto
            stream.codec.vcodec.clone().unwrap_or_default(),
            stream.codec.acodec.clone().unwrap_or_default(),
        );
    }
}
