use clap::Parser;
use miette::{Context, IntoDiagnostic, Result};
use reqwest::Client;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;
use url::Url;

#[derive(Parser)]
#[command(name = "noway")]
#[command(about = "Download archived pages from the Wayback Machine")]
struct Args {
    #[arg(help = "The URL to fetch archived versions of")]
    url: String,

    #[arg(short, long, help = "Output directory for downloaded files")]
    output: Option<String>,

    #[arg(
        short,
        long,
        default_value = "prefix",
        help = "Match type for URL search"
    )]
    match_type: String,

    #[arg(
        short,
        long,
        default_value = "5",
        help = "Maximum concurrent downloads"
    )]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let output_dir = args.output.unwrap_or_else(|| {
        let mut generator = names::Generator::default();
        generator.next().unwrap()
    });

    fs::create_dir_all(&output_dir)
        .into_diagnostic()
        .context(format!("Failed to create output directory: {}", output_dir))?;

    println!(
        "Fetching archived URLs for {} using CDX API",
        args.url
    );

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .into_diagnostic()?;

    let capture_urls = get_wayback_cdx_urls(&client, &args.url, &args.match_type).await?;

    if capture_urls.is_empty() {
        println!("No archived URLs found.");
        return Ok(());
    }

    let total = capture_urls.len();
    println!("Found {} archived URLs.", total);

    let semaphore = Arc::new(Semaphore::new(args.concurrency));
    let client = Arc::new(client);
    let output_dir = Arc::new(output_dir);
    let failed_urls = Arc::new(tokio::sync::Mutex::new(Vec::new()));

    let tasks: Vec<_> = capture_urls
        .into_iter()
        .enumerate()
        .map(|(i, url)| {
            let semaphore = Arc::clone(&semaphore);
            let client = Arc::clone(&client);
            let output_dir = Arc::clone(&output_dir);
            let failed_urls = Arc::clone(&failed_urls);

            tokio::spawn(async move {
                let _permit = semaphore.acquire().await.unwrap();
                println!("Downloading {}/{}: {}", i + 1, total, url);

                match download_html(&client, &url, &output_dir).await {
                    Ok(filename) => {
                        println!("Successfully downloaded: {}", filename);
                    }
                    Err(e) => {
                        println!("Failed to download {}: {}", url, e);
                        failed_urls.lock().await.push(url);
                    }
                }
            })
        })
        .collect();

    for task in tasks {
        let _ = task.await;
    }

    let failed_urls = failed_urls.lock().await;
    if !failed_urls.is_empty() {
        let log_file = PathBuf::from(&*output_dir).join("failed_urls.txt");
        let failed_content = failed_urls.join("\n");
        fs::write(&log_file, failed_content).into_diagnostic()?;
        println!(
            "Some URLs failed to download. Check {} for details.",
            log_file.display()
        );
    }

    println!("Download completed.");
    Ok(())
}

async fn get_wayback_cdx_urls(
    client: &Client,
    base_url: &str,
    match_type: &str,
) -> Result<Vec<String>> {
    let encoded_url = urlencoding::encode(base_url);
    let cdx_api_url = format!(
        "https://web.archive.org/cdx/search/cdx?url={}&matchType={}&filter=statuscode:200&output=json",
        encoded_url, match_type
    );

    let response = client
        .get(&cdx_api_url)
        .send()
        .await
        .into_diagnostic()
        .context("Failed to fetch CDX API")?;

    let data: Vec<Vec<Value>> = response
        .json()
        .await
        .into_diagnostic()
        .context("Failed to parse CDX JSON")?;

    if data.len() <= 1 {
        println!("No captures found in CDX API response.");
        return Ok(Vec::new());
    }

    let headers = &data[0];
    let timestamp_idx = headers
        .iter()
        .position(|h| h.as_str() == Some("timestamp"))
        .context("timestamp field not found")?;
    let original_url_idx = headers
        .iter()
        .position(|h| h.as_str() == Some("original"))
        .context("original field not found")?;

    let mut capture_urls = Vec::new();
    for row in data.iter().skip(1) {
        let timestamp = row[timestamp_idx].as_str().context("Invalid timestamp")?;
        let original_url = row[original_url_idx].as_str().context("Invalid URL")?;
        let capture_url = format!("https://web.archive.org/web/{}/{}", timestamp, original_url);
        capture_urls.push(capture_url);
    }

    Ok(capture_urls)
}

async fn download_html(client: &Client, url: &str, output_dir: &str) -> Result<String> {
    let response = client
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .into_diagnostic()
        .context("Failed to fetch URL")?;

    let html = response
        .text()
        .await
        .into_diagnostic()
        .context("Failed to read response")?;

    let timestamp = url
        .split("/web/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .unwrap_or("unknown");

    let parsed_url = Url::parse(url).into_diagnostic()?;
    let path = parsed_url
        .path()
        .replace(['/', ':'], "_");
    let filename = format!("{}_{}.html", timestamp, path);
    let filepath = PathBuf::from(output_dir).join(&filename);

    let mut file = File::create(&filepath)
        .await
        .into_diagnostic()
        .context("Failed to create file")?;
    file.write_all(html.as_bytes())
        .await
        .into_diagnostic()
        .context("Failed to write file")?;

    Ok(filename)
}
