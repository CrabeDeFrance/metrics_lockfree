#[derive(Debug, Clone)]
pub enum InternalMetricType<'a> {
    Counter(&'a str),
    Gauge(&'a str),
}

#[derive(Debug, Clone)]
pub enum InternalMetricTypeString {
    Counter(String),
    Gauge(String),
}

impl<'a> From<&InternalMetricType<'a>> for InternalMetricTypeString {
    fn from(value: &InternalMetricType<'a>) -> Self {
        match value {
            InternalMetricType::Counter(s) => InternalMetricTypeString::Counter(String::from(*s)),
            InternalMetricType::Gauge(s) => InternalMetricTypeString::Gauge(String::from(*s)),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
}
