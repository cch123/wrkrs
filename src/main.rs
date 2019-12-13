use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::signal;
use tokio::time::delay_for;

use std::sync::atomic::AtomicI32;
use std::sync::atomic::AtomicI64;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use futures::future::join_all;

use std::time::{Duration, Instant};

/*
async fn connect(addr: &str) -> io::Result<TcpStream> {
    match TcpStream::connect(addr).await {
        Ok(stream) => {
            //debug!("connected to {}", addr);
            Ok(stream)
        }
        Err(e) => {
            if e.kind() != io::ErrorKind::TimedOut {
                //error!("unknown connect error: '{}'", e);
            }
            Err(e)
        }
    }
}
*/

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let req_str = b"GET / HTTP/1.1
Host: localhost:9090

";
    //Connection: keep-alive

    // Create the runtime
    //let mut rt = Runtime::new()?;
    /*let mut rt = runtime::Builder::new()
    .threaded_scheduler()
    .num_threads(15)
    .enable_all()
    .build()
    .unwrap();
    */

    let connection_num = 100i32;
    let stopped = Arc::new(AtomicBool::new(false));

    let (counter, bytes_counter, total_time) = (
        Arc::new(AtomicI32::new(0)),
        Arc::new(AtomicI32::new(0)),
        Arc::new(AtomicI64::new(0)),
    );
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

    // for requests
    //let (mut tx, mut rx) = mpsc::channel(connection_num);
    let mut handles = vec![];

    (0..connection_num).for_each(|_| {
        //let tx = tx.clone();
        let (counter, bytes_counter, total_time) =
            (counter.clone(), bytes_counter.clone(), total_time.clone());
        let (mut counter_inner, mut bytes_counter_inner) = (0, 0);
        let stopped_clone = stopped.clone();
        let h = tokio::spawn(async move {
            let mut read_buffer = [0u8; 1024];
            let mut stream = TcpStream::connect("127.0.0.1:9090").await.unwrap();

            let thread_start = Instant::now();
            let (mut min_resp_time, mut max_resp_time) = (std::u128::MAX, std::u128::MIN);
            while !stopped_clone.load(Ordering::SeqCst) {
                let request_start = Instant::now();
                match stream.write(req_str).await {
                    Ok(_) => match stream.read(&mut read_buffer).await {
                        Ok(n) => {
                            counter_inner += 1;
                            bytes_counter_inner += n as i32;
                            //println!( "read {} bytes, {:?}", n, String::from_utf8(read_buffer[..n].to_vec()) );
                            let elapsed = request_start.elapsed().as_micros();
                            min_resp_time = min_resp_time.min(elapsed);
                            max_resp_time = max_resp_time.max(elapsed);
                        }
                        Err(_) => {}
                    },
                    Err(e) => println!("{}", e),
                };
            }
            //println!("avg resp: {} ", (thread_start.elapsed().as_millis() as f64)/(counter_inner as f64));

            // stats update
            counter.fetch_add(counter_inner, Ordering::Relaxed);
            bytes_counter.fetch_add(bytes_counter_inner, Ordering::Relaxed);
            total_time.fetch_add(thread_start.elapsed().as_millis() as i64, Ordering::Relaxed);
        });
        handles.push(h);
    });

    join_all(handles).await;

    println!("{:?}", now.elapsed());
    //println!("{:?}", total_time);

    // 不 join 的话，其实内部的 future 们还没有运行完
    println!(
        "total requests : {:?}; total bytes : {:?}",
        counter, bytes_counter
    );

    println!(
        "avg resp time : {:?}",
        (total_time.load(Ordering::Relaxed) as f64) / (counter.load(Ordering::Relaxed) as f64)
    );

    println!(
        "requests per second : {}",
        ((counter.load(Ordering::Relaxed) as f64) / (now.elapsed().as_secs() as f64)) as i32
    );

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
fn report() {}

/*
hey 的 report
总的统计，把请求的响应排好序以后就很简单了
Summary:
  Total:	5.0023 secs
  Slowest:	0.0159 secs
  Fastest:	0.0001 secs
  Average:	0.0007 secs
  Requests/sec:	68062.1232

  Total data:	1702340 bytes
  Size/request:	5 bytes

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

请求延迟分布，看 99 分位判断系统的延迟情况
Latency distribution:
  10% in 0.0005 secs
  25% in 0.0006 secs
  50% in 0.0007 secs
  75% in 0.0009 secs
  90% in 0.0010 secs
  95% in 0.0010 secs
  99% in 0.0015 secs

扩展的延迟展示，这个可以一并记录到请求的结构体里
Details (average, fastest, slowest):
  DNS+dialup:	0.0000 secs, 0.0001 secs, 0.0159 secs
  DNS-lookup:	0.0000 secs, 0.0000 secs, 0.0013 secs
  req write:	0.0000 secs, 0.0000 secs, 0.0030 secs
  resp wait:	0.0007 secs, 0.0001 secs, 0.0089 secs
  resp read:	0.0000 secs, 0.0000 secs, 0.0152 secs

Status code distribution:
  [200]	340468 responses
*/
fn report2() {}
