use futures::executor::LocalPool;
use futures::task::SpawnExt;

fn main() {
    let thread_num = 10;
    let connection_per_thread = 10;
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
