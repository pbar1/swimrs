use std::{thread, time::Duration as StdDurations};

use metrics::{
    decrement_gauge, gauge, histogram, increment_counter, increment_gauge, register_counter,
    register_histogram,
};
use metrics_exporter_prometheus::PrometheusBuilder;
use metrics_util::MetricKindMask;
use rand::{thread_rng, Rng};

fn main() {
    tracing_subscriber::fmt::init();

    let builder = PrometheusBuilder::new();
    builder
        .idle_timeout(
            MetricKindMask::COUNTER | MetricKindMask::HISTOGRAM,
            Some(StdDurations::from_secs(10)),
        )
        .install()
        .expect("failed to install Prometheus recorder");

    // We register these metrics, which gives us a chance to specify a description for them.  The
    // Prometheus exporter records this description and adds it as HELP text when the endpoint is
    // scraped.
    //
    // Registering metrics ahead of using them is not required, but is the only way to specify the
    // description of a metric.
    register_counter!(
        "tcp_server_loops",
        "The iterations of the TCP server event loop so far."
    );
    register_histogram!(
        "tcp_server_loop_delta_ns",
        "The time taken for iterations of the TCP server event loop."
    );

    increment_counter!("idle_metric");
    gauge!("testing", 42.0);

    // Loop over and over, pretending to do some work.
    loop {
        increment_counter!("tcp_server_loops", "system" => "foo");

        let increment_gauge = thread_rng().gen_bool(0.75);
        if increment_gauge {
            increment_gauge!("lucky_iterations", 1.0);
        } else {
            decrement_gauge!("lucky_iterations", 1.0);
        }

        thread::sleep(StdDuration::from_millis(750));
    }
}
