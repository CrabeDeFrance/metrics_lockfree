use prometheus::{Encoder, TextEncoder};
use std::{
    net::SocketAddr,
    sync::{LazyLock, RwLock},
    thread,
};
use tiny_http::{Header, Request, Response, Server as HTTPServer, StatusCode};

use std::io::{Error, ErrorKind, Result};

type ExportFn = fn() -> Vec<prometheus::proto::MetricFamily>;

static METRICS_EXPORT_FN: LazyLock<RwLock<Vec<ExportFn>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

pub struct Exporter {
    binding: SocketAddr,
    endpoint: String,
}

impl Exporter {
    pub fn builder(binding: SocketAddr) -> Self {
        Self {
            binding,
            endpoint: String::from("/metrics"),
        }
    }

    pub fn register(f: ExportFn) {
        if let Ok(mut metrics) = METRICS_EXPORT_FN.write() {
            metrics.push(f);
        } else {
            // todo
        }
    }

    /*
        let binding = "127.0.0.1:9186".parse().unwrap();
        let exporter = metrics_lockfree::Exporter::start(binding).unwrap();
    */

    pub fn start(binding: SocketAddr) -> Result<()> {
        let exporter = Self::builder(binding);

        let server = HTTPServer::http(exporter.binding).map_err(|e| {
            Error::new(
                ErrorKind::ConnectionAborted,
                format!("Can't start http server: {e}"),
            )
        })?;

        let endpoint = exporter.endpoint.clone();

        let _th = thread::spawn(move || {
            for request in server.incoming_requests() {
                let _err = if request.url() == endpoint {
                    Self::handler_metrics(request)
                } else {
                    Self::handler_redirect(request, &endpoint)
                };
            }
        });

        Ok(())
    }

    fn handler_metrics(request: Request) -> Result<()> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let mut metric_families = vec![];

        // fill metric families vector calling all metrics functions
        match METRICS_EXPORT_FN.read() {
            Ok(metrics) => {
                metrics.iter().for_each(|f| {
                    let m = (f)();
                    m.into_iter().for_each(|m| metric_families.push(m));
                });

                encoder.encode(&metric_families, &mut buffer).map_err(|e| {
                    Error::new(ErrorKind::InvalidData, format!("Can't encode metrics: {e}"))
                })?;

                let response = Response::from_data(buffer).with_status_code(StatusCode(200));
                request
                    .respond(response)
                    .map_err(|e| Error::new(e.kind(), format!("Can't send response: {e}")))
            }
            Err(e) => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Poison error: {e}"),
            )),
        }
    }

    fn handler_redirect(request: Request, endpoint: &str) -> Result<()> {
        let response = Response::from_string(format!("try {endpoint} for metrics\n"))
            .with_status_code(301)
            .with_header(Header {
                field: "Location"
                    .parse()
                    .expect("can not parse location header field. this should never fail"),
                value: ascii::AsciiString::from_ascii(endpoint)
                    .expect("can not parse header value. this should never fail"),
            });

        request
            .respond(response)
            .map_err(|e| Error::new(e.kind(), format!("Can't send redirect: {e}")))
    }
}
