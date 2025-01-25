use prometheus::proto::{Counter, Gauge, LabelPair, Metric, MetricFamily, MetricType};
use protobuf::RepeatedField;

pub fn prometheus_metric_family_build(
    ty: crate::types::MetricType,
    name: &str,
    value: u64,
    tags: Option<&[(String, String)]>,
) -> prometheus::proto::MetricFamily {
    let mut m = MetricFamily::new();
    m.set_name(name.to_owned());

    let m = match ty {
        crate::types::MetricType::Counter => {
            let mut counter = Counter::new();
            counter.set_value(value as f64);
            let mut metric = Metric::new();
            metric.set_counter(counter);

            m.set_field_type(MetricType::COUNTER);
            m.mut_metric().push(metric);
            m
        }
        crate::types::MetricType::Gauge => {
            let mut gauge = Gauge::new();
            gauge.set_value(value as f64);
            let mut metric = Metric::new();
            metric.set_gauge(gauge);

            m.set_field_type(prometheus::proto::MetricType::GAUGE);
            m.mut_metric().push(metric);
            m
        }
        crate::types::MetricType::CounterWithTags => {
            let mut counter = Counter::new();
            counter.set_value(value as f64);
            let mut metric = Metric::new();
            metric.set_counter(counter);

            if let Some(tags) = tags {
                let mut labels = vec![];
                tags.iter().for_each(|(k, v)| {
                    let mut label = LabelPair::new();
                    label.set_name(k.to_owned());
                    label.set_value(v.to_owned());
                    labels.push(label);
                });
                metric.set_label(RepeatedField::from_vec(labels));
            }

            m.set_field_type(MetricType::COUNTER);
            m.mut_metric().push(metric);
            m
        }
    };

    m
}
