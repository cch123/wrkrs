use futures::executor::LocalPool;
use futures::task::SpawnExt;
use async_std::net::{TcpStream};
use std::net::SocketAddr;
use async_std::io;//::{self};

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
    let addr = "localhost:9999";

    let mut thread_handle_list= vec![];
    (0..thread_num).for_each(|_|{
        let h = std::thread::spawn(move || {
            let mut pool = LocalPool::new();
            let mut spawner = pool.spawner();
            (0..connection_per_thread).for_each(|i|{
                println!("{}", i);
                spawner.spawn(async move {
                    // load test task
                    println!("{}", "internal");

                });
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
