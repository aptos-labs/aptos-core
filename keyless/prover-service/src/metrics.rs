use prometheus::{Encoder, IntCounter, Registry, TextEncoder, Histogram, HistogramOpts};

pub struct ProverServerMetrics {
    pub request_counter : IntCounter,
    pub prover_time : Histogram,
    pub groth16_time : Histogram,
    pub witness_generation_time : Histogram,
    pub response_time : Histogram,
    pub registry: Registry
}

impl ProverServerMetrics {
    pub fn new() -> Self {
        let request_counter = IntCounter::new("request_counter", "Total number of requests").unwrap();


        let prover_time = Histogram::with_opts(
            HistogramOpts::new("prover_time", "Prover time in seconds")
            .buckets(vec![1.0, 2.0, 3.0])
            )
            .unwrap();

        let groth16_time = Histogram::with_opts(
            HistogramOpts::new("groth16_time", "Time to run Groth16 in seconds")
            .buckets(vec![1.0, 2.0, 3.0])
            )
            .unwrap();

        let witness_generation_time = Histogram::with_opts(
            HistogramOpts::new("witness_generation_time", "Witness generation time in seconds")
            .buckets(vec![0.5, 1.0, 2.0])
            )
            .unwrap();

        let response_time = Histogram::with_opts(
            HistogramOpts::new("response_time", "Response time in seconds")
            .buckets(vec![1.0, 2.0, 3.0])
            )
            .unwrap();

        let registry = Registry::new();
        registry.register(Box::new(request_counter.clone())).unwrap();
        registry.register(Box::new(prover_time.clone())).unwrap();
        registry.register(Box::new(groth16_time.clone())).unwrap();
        registry.register(Box::new(witness_generation_time.clone())).unwrap();
        registry.register(Box::new(response_time.clone())).unwrap();

        ProverServerMetrics { request_counter , prover_time, groth16_time, witness_generation_time, response_time, registry }
    }

    pub fn encode_as_string(&self) -> String {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();

    let metric_families = self.registry.gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    String::from_utf8(buffer).unwrap()
    }

}

