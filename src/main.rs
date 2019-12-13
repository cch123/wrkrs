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
                .takes_value(true)
        )
        .get_matches();

    let connection_num = matches
        .value_of("conn_num")
        .unwrap()
        .parse::<i32>()
        .unwrap();

    let addr = matches
        .value_of("addr")
        .unwrap().to_string();

    let req_str = b"GET / HTTP/1.1
Host: localhost:9090

";
    //Connection: keep-alive

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

/*
wrk 的 report
展示的信息其实比较 old fashion
Running 5s test @ http://localhost:9090
  12 threads and 120 connections
  Thread Stats   Avg      Stdev     Max   +/- Stdev
    Latency     1.44ms  208.79us   3.67ms   80.44%
    Req/Sec     6.96k   282.18     8.04k    77.29%
  423744 requests in 5.10s, 45.26MB read
Requests/sec:  83063.44
Transfer/sec:      8.87MB
*/

/*
这个直方图实际上是分了 11 个 bucket
响应延迟排好序，然后按照顺序把计数计到相应的 bucket 里就行了，没什么难度
可以考虑用 tui-rs 来展示
Response time histogram:
  0.000 [1]	|
  0.002 [338512]	|■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.003 [1778]	|
  0.005 [123]	|
  0.006 [28]	|
  0.008 [15]	|
  0.010 [5]	|
  0.011 [3]	|
  0.013 [2]	|
  0.014 [0]	|
  0.016 [1]	|

扩展的延迟展示，这个可以一并记录到请求的结构体里
Details (average, fastest, slowest):
  DNS+dialup:	0.0000 secs, 0.0001 secs, 0.0159 secs
  DNS-lookup:	0.0000 secs, 0.0000 secs, 0.0013 secs
  req write:	0.0000 secs, 0.0000 secs, 0.0030 secs
  resp wait:	0.0007 secs, 0.0001 secs, 0.0089 secs
  resp read:	0.0000 secs, 0.0000 secs, 0.0152 secs

*/
fn report(total_time: Duration, mut resp_list: MutexGuard<Vec<Resp>>) {
    println!("Running benchmark for: \n  {:?}\n", total_time);

    resp_list.sort_by(|a, b| a.latency.cmp(&b.latency));

    println!(
        "Latency stats:\n  avg latency : {:?} ms\n  min latency : {:?}\n  max latency : {:?}\n",
        (resp_list
            .iter()
            .map(|e| { e.latency.as_nanos() })
            .sum::<u128>()
            / 1000000u128) as f64
            / (resp_list.len() as f64),
        resp_list.first().unwrap().latency,
        resp_list.last().unwrap().latency,
    );

    println!("Latency distribution:");

    let pos = vec![10, 25, 50, 75, 90, 95, 99];
    pos.iter().for_each(|p| {
        let idx = resp_list.len() * p / 100;
        println!("  {}% in {:?}", p, resp_list.get(idx).unwrap().latency);
    });

    println!("\nSummary:");
    println!(
        "  Total Requests: {:?}  Average QPS: {:?}  Total Transfer: {:?} MB/s",
        resp_list.len(),
        resp_list.len() / 5,
        resp_list.iter().map(|e|{e.byte_count}).sum::<usize>() as f64/ 5000000 as f64,
    );
}
