use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::signal;
use tokio::sync::Mutex;
use tokio::sync::MutexGuard;
use tokio::time::delay_for;

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use futures::future::join_all;

use std::time::{Duration, Instant};

use clap::{App, Arg};

use ansi_term::Colour::Blue;


#[derive(Debug)]
struct Resp {
    latency: std::time::Duration,
    byte_count: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("wrk-rs")
        .version("1.0")
        .author("Xargin")
        .about("bench your app")
        .arg(
            Arg::with_name("conn_num")
                .short("c")
                .help("set connection number")
                .default_value("12")
                .takes_value(true), // 没有 takes_value 的话，可能会读不到
        )
        .arg(
            Arg::with_name("addr")
                .short("a")
                .help("set request address")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let connection_num = matches
        .value_of("conn_num")
        .unwrap()
        .parse::<i32>()
        .unwrap();

    let addr = matches.value_of("addr").unwrap().to_string();

    let req_str = b"GET / HTTP/1.1
Host: localhost:9090
Connection: keep-alive

";

    let stopped = Arc::new(AtomicBool::new(false));

    println!("Running in {} connections", connection_num);

    let now = Instant::now();

    // for timeout
    let stopped_c = stopped.clone();
    tokio::spawn(async move {
        delay_for(Duration::from_secs(5)).await;
        stopped_c.store(true, Ordering::SeqCst);
    });

    // for signal
    let stopped_s = stopped.clone();
    tokio::spawn(async move {
        signal::ctrl_c().await.unwrap();
        stopped_s.store(true, Ordering::SeqCst);
    });

    let mut handles = vec![];
    let resp_list_summary = Arc::new(Mutex::new(Vec::new()));

    // for requests
    (0..connection_num).for_each(|_| {
        let resp_list_summary = resp_list_summary.clone();
        let stopped_clone = stopped.clone();
        let addr = addr.clone();
        let h = tokio::spawn(async move {
            let mut read_buffer = [0u8; 1024];
            let mut stream = TcpStream::connect(addr).await.unwrap();

            let mut resp_list = vec![];

            while !stopped_clone.load(Ordering::SeqCst) {
                let request_start = Instant::now();
                match stream.write(req_str).await {
                    Ok(_) => match stream.read(&mut read_buffer).await {
                        Ok(n) => {
                            resp_list.push(Resp {
                                latency: request_start.elapsed(),
                                byte_count: n,
                            });
                        }
                        Err(_) => {}
                    },
                    Err(e) => println!("{}", e),
                };
            }

            // stats update
            resp_list_summary.lock().await.append(&mut resp_list);
        });
        handles.push(h);
    });

    // 不 join 的话，其实内部的 future 们还没有运行完
    join_all(handles).await;

    report(now.elapsed(), resp_list_summary.lock().await);

    Ok(())
}

fn report(total_time: Duration, mut resp_list: MutexGuard<Vec<Resp>>) {
    println!("\n{}\n  {:?}\n", Blue.paint("Running Benchmark for:"), total_time);

    resp_list.sort_by(|a, b| a.latency.cmp(&b.latency));

    let (min_latency, max_latency) = (
        resp_list.first().unwrap().latency,
        resp_list.last().unwrap().latency,
    );

    println!("{}", Blue.paint("Latency Stats:"));
    println!(
        "  AVG Latency : {:?} ms\n  Min Latency : {:?}\n  Max Latency : {:?}\n",
        (resp_list
            .iter()
            .map(|e| { e.latency.as_nanos() })
            .sum::<u128>()
            / 1000000u128) as f64
            / (resp_list.len() as f64),
        min_latency,
        max_latency,
    );

    println!("{}", Blue.paint("Latency Distribution:"));

    let pos = vec![10, 25, 50, 75, 90, 95, 99];
    pos.iter().for_each(|p| {
        let idx = resp_list.len() * p / 100;
        println!("  {}% in {:?}", p, resp_list.get(idx).unwrap().latency);
    });

    let mut time_gap = vec![];
    (0..=10).for_each(|i| {
        time_gap.push(
            min_latency.as_nanos() + i * (max_latency.as_nanos() - min_latency.as_nanos()) / 10,
        );
    });

    let mut resp_time_dist = vec![];
    let mut cursor = 0;
    let mut max_dist_block_count = 0;

    time_gap.iter().for_each(|t| {
        let mut counter = 0;
        while cursor < resp_list.len() {
            let latency = resp_list.get(cursor).unwrap().latency.as_nanos();
            if *t >= latency {
                counter += 1;
                cursor += 1;
                continue;
            } else {
                break;
            }
        }
        max_dist_block_count = counter.max(max_dist_block_count);
        resp_time_dist.push(counter)
    });

    println!("{}", Blue.paint("\nResponse Distribution:"));
    resp_time_dist.iter().enumerate().for_each( |(idx, cnt)|{
        println!(
            "  {:<10}  {:<10} |{:<}",
            time_cost = format!("{:?}", Duration::from_nanos(*time_gap.get(idx).unwrap() as u64)),
            cnt = format!("[{}]", cnt),
            dist = "■".repeat((cnt * 40 / max_dist_block_count) as usize)
        );
    });

    println!("{}", Blue.paint("\nSummary:"));
    println!(
        "  Total Requests: {:?}  Average QPS: {:?}  Total Transfer: {:?} MB/s",
        resp_list.len(),
        resp_list.len() / 5,
        resp_list.iter().map(|e| { e.byte_count }).sum::<usize>() as f64 / 5000000 as f64,
    );
}
