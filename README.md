# Load Testing Tool

This is a simple load testing tool written in Rust using the Tokio and Reqwest library. It allows you to perform load testing on a given URL by sending multiple concurrent HTTP requests and measuring various performance metrics.

## Features

- Send multiple concurrent HTTP requests to a specified URL.
- Measure various performance metrics, including Time To First Byte (TTFB), Time To Last Byte (TTLB), and total request time.
- Calculate percentiles (P95 and P99) for response times.
- Display statistics such as the number of successful and failed requests, requests per second (RPS), and more.
- Easy-to-use command-line interface for specifying the URL, the number of requests, and concurrency level.

## Usage

```bash
Usage: load-tester [OPTIONS] --url <URL> -n <NUMBER> -c <CONCURRENCY>

Options:
  -u, --url <URL>                  URL
  -n, --number <NUMBER>            Number of times to make request [default: 10]
  -c, --concurrency <CONCURRENCY>  Number of concurrent requests [default: 1]
  -h, --help                       Print help
  -V, --version                    Print version


cargo run -- -u https://httpbin.org/get -n 100 -c 10
```
