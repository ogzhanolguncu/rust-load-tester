use std::time::{Duration, Instant};

use clap::Parser;
use futures::future::join_all;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{self, StatusCode};
use tokio::{self};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// URL
    #[arg(short, long)]
    url: String,

    /// Number of times to make request
    #[arg(short, long, default_value_t = 10)]
    number: u8,

    /// Number of concurrent requests
    #[arg(short, long, default_value_t = 1)]
    concurrency: u8,
}

#[derive(Debug)]
struct Stats {
    ttlb: f32,
    ttfb: f32,
    total_time: f32,
    status: StatusCode,
}
struct LoadResult {
    number_of_successful_calls: u8,
    number_of_failed_calls: u8,
    stats: Vec<Stats>,
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    let args: Args = Args::parse();

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner());

    let url_to_test_against = args.url;
    let number_of_batches = args.number / args.concurrency;
    let remainder = args.number % args.concurrency;

    let mut final_result = LoadResult {
        number_of_failed_calls: 0,
        number_of_successful_calls: 0,
        stats: vec![],
    };

    spinner.enable_steady_tick(Duration::from_millis(100));
    spinner.set_message("Processing...");
    let test_start = Instant::now();

    for _ in 0..number_of_batches {
        final_result = process_batch(&url_to_test_against, args.concurrency, final_result).await;
    }
    // Process the remainder
    if remainder > 0 {
        final_result = process_batch(&url_to_test_against, remainder, final_result).await;
    }

    spinner.finish_with_message("Done!");
    let CalculatedStats {
        total_time: (total_min, total_max, total_mean),
        ttfb: (ttfb_min, ttfb_max, ttfb_mean),
        ttlb: (ttlb_min, ttlb_max, ttlb_mean),
    } = calculate_stats(&final_result);

    let test_end = Instant::now();

    let test_duration = test_end.duration_since(test_start).as_secs_f32();

    let rps = final_result.number_of_successful_calls as f32 / test_duration;

    // Usage
    let mut latencies = final_result
        .stats
        .iter()
        .map(|s| s.ttfb)
        .collect::<Vec<f32>>();
    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let p95 = calculate_percentiles(&latencies, 95.0);
    let p99 = calculate_percentiles(&latencies, 99.0);

    println!("Results:");
    println!(
        "Total Requests (2XX).......................: {}",
        final_result.number_of_successful_calls
    );
    println!(
        "Failed Requests (5XX).......................: {}",
        final_result.number_of_failed_calls
    );
    println!("Request Per Sec (RPS).......................: {}", rps);
    println!();
    println!();
    println!("P95.......................: {}", p95);
    println!("P99.......................: {}", p99);
    println!();
    println!();
    println!(
        "Total Request Time (s) (Min, Max, Mean).....: {}, {}, {},",
        total_min, total_max, total_mean
    );
    println!(
        "Time to First Byte (s) (Min, Max, Mean).....: {}, {}, {},",
        ttfb_min, ttfb_max, ttfb_mean
    );
    println!(
        "Time to Last Byte (s) (Min, Max, Mean).....: {}, {}, {},",
        ttlb_min, ttlb_max, ttlb_mean
    );

    Ok(())
}

async fn make_request(url: &str) -> Result<Stats, reqwest::Error> {
    let start = Instant::now();

    // Start the request
    let res = reqwest::get(url).await?;
    let status = res.status();

    // Time to first byte (TTFB)
    let ttfb = start.elapsed().as_secs_f32();

    // Read the whole body
    let _ = res.bytes().await?;
    // Measure the time immediately after the body is fully read
    let body_end = Instant::now();

    // Time to last byte (TTLB)
    let ttlb = body_end.duration_since(start).as_secs_f32();

    let total_time = Instant::now().duration_since(start).as_secs_f32();

    Ok(Stats {
        ttlb,
        ttfb,
        total_time,
        status,
    })
}

async fn process_batch(url: &str, count: u8, mut result: LoadResult) -> LoadResult {
    let mut futures = Vec::new();
    for _ in 0..count {
        futures.push(make_request(url));
    }

    let calls: Vec<Result<Stats, reqwest::Error>> = join_all(futures).await;
    for call in calls {
        match call {
            Ok(resp) if resp.status.is_success() => {
                result.number_of_successful_calls += 1;
                result.stats.push(resp);
            }

            Ok(_) => result.number_of_failed_calls += 1,
            Err(_) => result.number_of_failed_calls += 1,
        }
    }

    result
}

#[derive(Debug)]
struct CalculatedStats {
    ttfb: (f32, f32, f32),
    ttlb: (f32, f32, f32),
    total_time: (f32, f32, f32),
}

fn calculate_stats(result: &LoadResult) -> CalculatedStats {
    let mut ttfb_min = f32::MAX;
    let mut ttfb_max = f32::MIN;
    let ttfb_mean = calculate_mean(&result.stats, |x| x.ttfb);

    for stat in &result.stats {
        ttfb_min = ttfb_min.min(stat.ttfb);
        ttfb_max = ttfb_max.max(stat.ttfb);
    }

    let mut ttlb_min = f32::MAX;
    let mut ttlb_max = f32::MIN;
    let ttlb_mean = calculate_mean(&result.stats, |x| x.ttlb);

    for stat in &result.stats {
        ttlb_min = ttlb_min.min(stat.ttlb);
        ttlb_max = ttlb_max.max(stat.ttlb);
    }

    let mut total_min = f32::MAX;
    let mut total_max = f32::MIN;
    let total_mean = calculate_mean(&result.stats, |x| x.total_time);

    for stat in &result.stats {
        total_min = total_min.min(stat.total_time);
        total_max = total_max.max(stat.total_time);
    }

    CalculatedStats {
        ttfb: (
            truncate_to_two_decimals(ttfb_min),
            truncate_to_two_decimals(ttfb_max),
            truncate_to_two_decimals(ttfb_mean.unwrap_or_default()),
        ),
        ttlb: (
            truncate_to_two_decimals(ttlb_min),
            truncate_to_two_decimals(ttlb_max),
            truncate_to_two_decimals(ttlb_mean.unwrap_or_default()),
        ),
        total_time: (
            truncate_to_two_decimals(total_min),
            truncate_to_two_decimals(total_max),
            truncate_to_two_decimals(total_mean.unwrap_or_default()),
        ),
    }
}

fn calculate_mean<F>(numbers: &[Stats], value_extractor: F) -> Option<f32>
where
    F: FnMut(&Stats) -> f32,
{
    let sum: f32 = numbers.iter().map(value_extractor).sum();
    let count = numbers.len();

    if count > 0 {
        Some(sum / count as f32)
    } else {
        None
    }
}

fn truncate_to_two_decimals(num: f32) -> f32 {
    (num * 100.0).trunc() / 100.0
}

fn calculate_percentiles(latencies: &Vec<f32>, percentile: f32) -> f32 {
    let len = latencies.len();
    if len == 0 {
        return 0.0;
    }
    let index = (percentile / 100.0 * (len as f32 - 1.0)).round() as usize;
    *latencies.get(index).unwrap_or(&0.0)
}
