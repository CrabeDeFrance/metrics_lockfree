use std::thread::{sleep, spawn};
use std::time::Duration;

use lockfree_metrics_macros::Metrics;

#[derive(Metrics)]
pub struct MyMetrics {
    // doc type c
    c: Counter,
    // doc type d
    //#[max_cardinality = "1000"]
    d: Gauge,
}

use prometheus::proto::{Counter, Gauge, Metric, MetricFamily, MetricType};
use prometheus::TextEncoder;

fn main() {
    let mut thread1 = MyMetrics::new();

    let t1 = spawn(move || loop {
        thread1.add_c(1);
        thread1.set_d(1);
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

        let mut metricfamilies = vec![];

        (0..metrics.len()).for_each(|idx| {
            let mut metricfamily = MetricFamily::new();
            metricfamily.set_name(metrics[idx].clone());
            metricfamily.set_field_type(MetricType::COUNTER);

            let mut counter = Counter::new();
            counter.set_value(values[idx] as f64);
            let mut metric = Metric::new();
            metric.set_counter(counter);

            metricfamily.mut_metric().push(metric);

            metricfamilies.push(metricfamily);
        });

        let encoder = TextEncoder::new();
        let encoded = encoder.encode_to_string(&metricfamilies).unwrap();

        //    println!("{}: {}", metrics[idx], values[idx]);
        println!("{encoded}");
    });

    let _ = t1.join();
    let _ = t2.join();
    let _ = stat_thread.join();
}
