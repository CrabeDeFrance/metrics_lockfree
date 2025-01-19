use std::time::Instant;

use metrics::{counter, histogram};
use metrics_exporter_prometheus::PrometheusBuilder;

fn main() {
    println!("Hello, world!");

    let start = Instant::now();
    //let row_count = run_query(query);
    let delta = start.elapsed();

    // First, create a builder.
    //
    // The builder can configure many aspects of the exporter, such as changing the
    // listen address, adjusting how histograms will be reported, changing how long
    // metrics can be idle before being removed, and more.
    let builder = PrometheusBuilder::new();

    let recorder = builder.build_recorder();

    let ret = metrics::set_default_local_recorder(&recorder);

    histogram!("process.query_time").record(delta);
    counter!("process.query_row_count").increment(1);
}
