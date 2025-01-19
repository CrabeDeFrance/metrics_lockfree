use std::thread::{sleep, spawn};
use std::time::Duration;

use lockfree_metrics_macros::Metrics;

#[derive(Metrics)]
pub struct MyMetrics {
    c: u64,
    d: u64,
}

fn main() {
    println!("Hello, world!");

    let mut thread1 = MyMetrics::new();

    let t1 = spawn(move || loop {
        thread1.add_c(1);
        thread1.add_d(1);
        std::hint::black_box(&thread1);
    });

    let mut thread2 = MyMetrics::new();

    let t2 = spawn(move || loop {
        thread2.add_c(1);
        std::hint::black_box(&thread2);
    });

    // thread de collect et d'agrégation
    let stat_thread = spawn(|| loop {
        sleep(Duration::from_secs(1));

        let factory = MyMetrics::read_lock().unwrap();
        let metrics = factory.metrics();
        let mut values = vec![0; metrics.len()];

        factory.thread().iter().for_each(|list| {
            list.iter()
                .enumerate()
                .for_each(|(idx, v)| values[idx] += v);
        });

        (0..metrics.len()).for_each(|idx| {
            println!("{}: {}", metrics[idx], values[idx]);
        });
    });

    let _ = t1.join();
    let _ = t2.join();
    let _ = stat_thread.join();
}
