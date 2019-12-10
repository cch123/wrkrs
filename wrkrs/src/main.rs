use async_std::io;
use async_std::net::TcpStream;
use async_std::prelude::*;
use futures::executor::LocalPool;
use futures::task::SpawnExt;
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};
use std::thread::spawn;

async fn connect(addr: &SocketAddr) -> io::Result<TcpStream> {
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

fn main() {
    let thread_num = 10;
    let connection_per_thread = 10;
    let addr = "localhost:9090".to_socket_addrs().unwrap().next().unwrap();
    let req_str = b"GET / HTTP/1.1
Accept: */*
Accept-Encoding: gzip, deflate
Connection: keep-alive
Host: localhost:9090
User-Agent: HTTPie/1.0.2

";

    let mut thread_handle_list = vec![];
    let start = Instant::now();

    (0..thread_num).for_each(|_| {
        let h = std::thread::spawn(move || {
            let mut pool = LocalPool::new();
            let spawner = pool.spawner();
            (0..connection_per_thread).for_each(|i| {
                println!("{}", i);
                let mut recv_bytes_total = 0;
                let mut counter = 0;
                spawner
                    .spawn(async move {
                        let mut read_buffer = [0u8; 1024];
                        let mut stream = connect(&addr).await.unwrap();
                        // load test task
                        // write request text here
                        while Instant::now() <= start + Duration::from_secs(5) {
                            counter += 1;
                            match stream.write_all(req_str).await {
                                Ok(_) => match stream.read(&mut read_buffer).await {
                                    Ok(n) => {
                                        recv_bytes_total += n;
                                        /*
                                        println!(
                                            "read {} bytes, {:?}",
                                            n,
                                            String::from_utf8(read_buffer[..n].to_vec())
                                        );
                                        */
                                    }
                                    Err(_) => {}
                                },
                                Err(e) => println!("{}", e),
                            };
                        }
                    })
                    .unwrap();
            });
            pool.run();
        });
        thread_handle_list.push(h);
    });

    for h in thread_handle_list {
        h.join().unwrap();
    }
    /*
      // 看一下为什么这里会报：
      error[E0507]: cannot move out of a shared reference
    --> src/main.rs:32:9
      thread_handle_list.iter().for_each(|h|{
          //h.join().unwrap();
          h.join().unwrap();
      });
      */
}
