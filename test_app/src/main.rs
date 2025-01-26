use std::thread::spawn;

use metrics_lockfree::{counter::Counter, gauge::Gauge};

use metrics_lockfree_macros::Metrics;

#[derive(Metrics)]
pub struct MyMetrics {
    // doc type c
    c: Counter,

    g: Gauge,
    ct: Counter<32>,
}

fn main() {
    let binding = "127.0.0.1:9186".parse().unwrap();
    metrics_lockfree::Exporter::start(binding).unwrap();

    let mut thread1 = MyMetrics::new().unwrap();
    let t1 = spawn(move || loop {
        thread1.c.add(1, None);
        thread1
            .ct
            .add(1, Some(&[("key_a".to_string(), "val_b".to_string())]));
        thread1.ct.add(
            1,
            Some(&[
                ("key_a".to_string(), "val_b".to_string()),
                ("key_b".to_string(), "val_c".to_string()),
            ]),
        );

        std::hint::black_box(&thread1);
    });

    let mut thread2 = MyMetrics::new().unwrap();
    let t2 = spawn(move || loop {
        thread2.c.add(1, None);
        thread2.g.set(1);

        // for tags
        thread2
            .ct
            .add(1, Some(&[("key_a".to_string(), "val_a".to_string())]));
        std::hint::black_box(&thread2);
    });

    let _ = t1.join();
    let _ = t2.join();
}
