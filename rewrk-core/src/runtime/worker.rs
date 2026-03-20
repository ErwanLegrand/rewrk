use std::borrow::Cow;
use std::mem;
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::future::join_all;
use http::Request;
use hyper::Body;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::{timeout_at, Instant as TokioInstant};

use crate::connection::{ProtocolConnection, ProtocolConnector};
use crate::producer::{Batch, Producer, ProducerActor, ProducerBatches};
use crate::recording::{CollectorMailbox, SampleFactory, SampleMetadata};
use crate::runtime::expected_interval::ExpectedIntervalTracker;
use crate::runtime::shutdown::ShutdownHandle;
use crate::utils::RuntimeTimings;
use crate::validator::ValidationError;
use crate::{ResponseValidator, Sample};

/// The outcome of a single `send()` call.
enum RequestResult {
    /// Request completed (successfully or with a non-fatal validation error).
    Ok,
    /// The connection was closed by the server; caller should reconnect.
    Reconnect,
}

/// The outcome of executing a full batch of requests.
enum BatchResult {
    /// Batch completed, continue with the next one.
    Continue,
    /// Connection lost mid-batch, need to reconnect before continuing.
    NeedsReconnect,
    /// Fatal error or producer exhausted, stop this connection.
    Stop,
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
type ConnectionTask = JoinHandle<RuntimeTimings>;
type WorkerGuard = flume::Receiver<()>;

#[derive(Clone)]
pub(crate) struct WorkerConfig<P, C>
where
    P: Producer + Clone,
    C: ProtocolConnector,
{
    /// The benchmarking connector.
    pub connector: C,
    /// The maximum number of retry attempts for connecting.
    pub retry_max: usize,
    /// The selected validator for the benchmark.
    pub validator: Arc<dyn ResponseValidator>,
    /// The sample results collector.
    pub collector: CollectorMailbox,
    /// The request batch producer.
    pub producer: P,
    /// The duration which should elapse before a sample
    /// is submitted to be processed.
    pub sample_window: Duration,
    /// The percentage threshold that the system must be
    /// waiting on the producer in order for a warning to be raised.
    ///
    /// This is useful in situations where you know the producer will
    /// take more time than normal and want to silence the warning.
    pub producer_wait_warning_threshold: f32,
}

/// Spawns N worker runtimes for executing search requests.
pub(crate) fn spawn_workers<P, C>(
    shutdown: ShutdownHandle,
    num_workers: usize,
    concurrency: usize,
    config: WorkerConfig<P, C>,
) -> WorkerGuard
where
    P: Producer + Clone,
    C: ProtocolConnector + 'static,
{
    // We use a channel here as a guard in order to wait for all workers to shutdown.
    let (guard, waiter) = flume::bounded(1);

    let per_worker_concurrency = concurrency / num_workers;
    let mut remaining_concurrency = concurrency - (per_worker_concurrency * num_workers);

    for worker_id in 0..num_workers {
        let concurrency_modifier = if remaining_concurrency != 0 {
            remaining_concurrency -= 1;
            1
        } else {
            0
        };
        let concurrency = per_worker_concurrency + concurrency_modifier;

        spawn_worker(
            worker_id,
            concurrency,
            guard.clone(),
            shutdown.clone(),
            config.clone(),
        );
    }

    waiter
}

/// Spawns a new runtime worker thread.
fn spawn_worker<P, C>(
    worker_id: usize,
    concurrency: usize,
    guard: flume::Sender<()>,
    handle: ShutdownHandle,
    config: WorkerConfig<P, C>,
) where
    P: Producer + Clone,
    C: ProtocolConnector + 'static,
{
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Create runtime");

    std::thread::Builder::new()
        .name(format!("rewrk-worker-{worker_id}"))
        .spawn(move || {
            debug!(worker_id = worker_id, "Spawning worker");
            rt.block_on(run_worker(worker_id, concurrency, handle, config));

            // Drop the guard explicitly to make sure it's not dropped
            // until after the runtime has completed.
            drop(guard);

            debug!(worker_id = worker_id, "Worker successfully shutdown");
        })
        .expect("Spawn thread");
}

/// Runs a worker task.
///
/// This acts as the main runtime entrypoint.
async fn run_worker<P, C>(
    worker_id: usize,
    concurrency: usize,
    shutdown: ShutdownHandle,
    config: WorkerConfig<P, C>,
) where
    P: Producer + Clone,
    C: ProtocolConnector + 'static,
{
    let (ready_tx, ready_rx) = oneshot::channel();
    let producer =
        ProducerActor::spawn(concurrency * 4, worker_id, config.producer, ready_rx)
            .await;
    let metadata = SampleMetadata { worker_id };
    let sample_factory =
        SampleFactory::new(config.sample_window, metadata, config.collector);

    let mut pending_futures = Vec::<ConnectionTask>::with_capacity(concurrency);
    for _ in 0..concurrency {
        let task_opt = create_worker_connection(
            worker_id,
            &config.connector,
            config.retry_max,
            shutdown.clone(),
            sample_factory.clone(),
            config.validator.clone(),
            producer.clone(),
        )
        .await;

        match task_opt {
            None => {
                info!(worker_id = ?worker_id, "Cleaning up futures and shutting down...");
                for pending in pending_futures {
                    pending.abort();
                }
                return;
            },
            Some(task) => {
                pending_futures.push(task);
            },
        }
    }

    // Begin benchmarking.
    let _ = ready_tx.send(());

    // Wait for all tasks to complete.
    let timings = join_all(pending_futures)
        .await
        .into_iter()
        .collect::<Result<RuntimeTimings, _>>()
        .unwrap_or_else(|e| {
            tracing::error!(
                worker_id = worker_id,
                error = ?e,
                "A connection task panicked; using zero timings for this worker."
            );
            RuntimeTimings::default()
        });

    info!(worker_id = worker_id, "Benchmark completed for worker.");

    let total_duration = timings.execute_wait_runtime + timings.producer_wait_runtime;
    let producer_wait_pct = (timings.producer_wait_runtime.as_secs_f32()
        / total_duration.as_secs_f32())
        * 100.0;

    if producer_wait_pct >= config.producer_wait_warning_threshold {
        warn!(
            worker_id = worker_id,
            producer_wait_pct = producer_wait_pct,
            request_execute_wait_duration = ?timings.execute_wait_runtime,
            producer_wait_duration = ?timings.producer_wait_runtime,
            total_runtime_duration = ?total_duration,
            "The system spent {producer_wait_pct:.2}% of it's runtime waiting for the producer.\
             Results may not be accurate."
        );
    }
}

/// Establish a connection using any [`ProtocolConnector`], with retry and timeout logic.
///
/// Returns `Ok(Some(conn))` on success, `Ok(None)` if the timeout elapsed without
/// any connection error, or `Err(e)` if all retry attempts failed or the timeout
/// elapsed after at least one error.
async fn connect_with_timeout<C>(
    connector: &C,
    dur: Duration,
    retry_max: usize,
) -> anyhow::Result<Option<C::Connection>>
where
    C: ProtocolConnector,
{
    let deadline = TokioInstant::now() + dur;
    let mut last_error: Option<anyhow::Error> = None;
    let mut attempts_left = retry_max;

    loop {
        let result = timeout_at(deadline, connector.connect()).await;

        match result {
            Err(_) => {
                return if let Some(error) = last_error {
                    Err(error)
                } else {
                    Ok(None)
                }
            },
            Ok(Err(e)) => {
                if attempts_left == 0 {
                    return Err(e);
                }

                attempts_left -= 1;
                last_error = Some(e);
                tokio::time::sleep(Duration::from_millis(500)).await;
            },
            Ok(Ok(connection)) => return Ok(Some(connection)),
        }
    }
}

async fn create_worker_connection<C>(
    worker_id: usize,
    connector: &C,
    retry_max: usize,
    shutdown: ShutdownHandle,
    sample_factory: SampleFactory,
    validator: Arc<dyn ResponseValidator>,
    producer: ProducerBatches,
) -> Option<ConnectionTask>
where
    C: ProtocolConnector + 'static,
{
    let connect_result =
        connect_with_timeout(connector, CONNECT_TIMEOUT, retry_max).await;
    let conn = match connect_result {
        Err(e) => {
            // We check this to prevent spam of the logs.
            if !shutdown.should_abort() {
                error!(worker_id = worker_id, error = ?e, "Failed to connect to server due to error, aborting.");
                shutdown.set_abort();
            }
            return None;
        },
        Ok(None) => {
            // We check this to prevent spam of the logs.
            if !shutdown.should_abort() {
                error!(worker_id = worker_id, "Worker failed to connect to server within {CONNECT_TIMEOUT:?}, aborting.");
                shutdown.set_abort();
            }
            return None;
        },
        Ok(Some(conn)) => conn,
    };

    let mut connection = WorkerConnection::new(
        conn,
        sample_factory,
        validator,
        producer,
        shutdown.clone(),
    );

    let connector = connector.clone();
    let fut = async move {
        while !shutdown.should_abort() {
            match connection.execute_next_batch().await {
                BatchResult::Continue => {},
                BatchResult::NeedsReconnect => {
                    debug!(
                        worker_id = worker_id,
                        "Connection closed by server, attempting reconnect..."
                    );
                    match connect_with_timeout(
                        &connector,
                        CONNECT_TIMEOUT,
                        retry_max,
                    )
                    .await
                    {
                        Ok(Some(new_conn)) => {
                            debug!(
                                worker_id = worker_id,
                                "Successfully reconnected."
                            );
                            connection.replace_connection(new_conn);
                        },
                        _ => {
                            warn!(
                                worker_id = worker_id,
                                "Reconnection failed, stopping this connection."
                            );
                            break;
                        },
                    }
                },
                BatchResult::Stop => break,
            }
        }

        // Submit the remaining sample.
        connection.submit_sample(0);

        connection.timings
    };

    Some(tokio::spawn(fut))
}

pub(crate) struct WorkerConnection<Conn: ProtocolConnection> {
    /// The protocol connection for benchmarking.
    conn: Conn,
    /// The sample factory for producing metric samples.
    sample_factory: SampleFactory,
    /// The current sample being populated with metrics.
    sample: Sample,
    /// The selected validator for the benchmark.
    validator: Arc<dyn ResponseValidator>,
    /// The request batch producer.
    producer: ProducerBatches,
    /// The point in time when the last sample was submitted to
    /// the collectors.
    last_sent_sample: Instant,
    /// A signal flag telling all workers to shutdown.
    shutdown: ShutdownHandle,
    /// Internal timings which are useful for debugging.
    timings: RuntimeTimings,
    /// A check for if the first batch has been received already.
    ///
    /// This is so that timings can be adjusted while waiting for
    /// benchmarking to start, which would otherwise skew results.
    is_first_batch: bool,
    /// Tracks the expected inter-request interval for Coordinated
    /// Omission correction, based on the running average of observed
    /// successful request latencies.
    ///
    /// **Bootstrap gap:** The first successful request on each connection
    /// is recorded only into the uncorrected histogram because no expected
    /// interval is available yet. This is expected behaviour — one sample
    /// per connection is negligible relative to a typical benchmark run.
    expected_interval: ExpectedIntervalTracker,
}

impl<Conn: ProtocolConnection> WorkerConnection<Conn> {
    /// Create a new worker instance
    fn new(
        conn: Conn,
        sample_factory: SampleFactory,
        validator: Arc<dyn ResponseValidator>,
        producer: ProducerBatches,
        shutdown: ShutdownHandle,
    ) -> Self {
        let sample = sample_factory.new_sample(0);
        let last_sent_sample = Instant::now();

        Self {
            conn,
            sample_factory,
            sample,
            validator,
            producer,
            last_sent_sample,
            shutdown,
            timings: RuntimeTimings::default(),
            is_first_batch: true,
            expected_interval: ExpectedIntervalTracker::new(),
        }
    }

    /// Sets the abort flag across workers.
    fn set_abort(&self) {
        self.shutdown.set_abort()
    }

    /// Submit the current sample to the collectors and create a new
    /// sample with a given tag.
    fn submit_sample(&mut self, next_sample_tag: usize) -> bool {
        let new_sample = self.sample_factory.new_sample(next_sample_tag);
        let old_sample = mem::replace(&mut self.sample, new_sample);
        if self.sample_factory.submit_sample(old_sample).is_err() {
            return false;
        }
        self.last_sent_sample = Instant::now();
        true
    }

    /// Gets the next batch from the producer and submits it to be executed.
    ///
    /// Returns a [`BatchResult`] indicating whether to continue, reconnect,
    /// or stop.
    async fn execute_next_batch(&mut self) -> BatchResult {
        let producer_start = Instant::now();
        let batch = match self.producer.recv_async().await {
            Ok(batch) => batch,
            // We've completed all batches.
            Err(_) => return BatchResult::Stop,
        };
        let producer_elapsed = producer_start.elapsed();

        if self.is_first_batch {
            self.is_first_batch = false;
        } else {
            self.timings.producer_wait_runtime += producer_elapsed;
        }

        let execute_start = Instant::now();
        let result = self.execute_batch(batch).await;
        self.timings.execute_wait_runtime += execute_start.elapsed();

        result
    }

    /// Replace the underlying protocol connection (used after reconnection).
    fn replace_connection(&mut self, new_conn: Conn) {
        self.conn = new_conn;
    }

    /// Executes a batch of requests to measure the metrics.
    async fn execute_batch(&mut self, batch: Batch) -> BatchResult {
        if self.sample.tag() != batch.tag {
            let success = self.submit_sample(batch.tag);

            if !success {
                self.set_abort();
                return BatchResult::Stop;
            }
        }

        for request in batch.requests {
            match self.send(request).await {
                Ok(RequestResult::Ok) => {},
                Ok(RequestResult::Reconnect) => {
                    return BatchResult::NeedsReconnect;
                },
                Err(e) => {
                    error!(error = ?e, "Worker encountered an error while benchmarking, aborting...");
                    self.set_abort();
                    return BatchResult::Stop;
                },
            }
        }

        BatchResult::Continue
    }

    /// Send a HTTP request and record the relevant metrics.
    async fn send(
        &mut self,
        request: Request<Body>,
    ) -> Result<RequestResult, hyper::Error> {
        let read_transfer_start = self.conn.usage().get_received_count();
        let write_transfer_start = self.conn.usage().get_written_count();
        let start = Instant::now();

        let (head, body) = match self.conn.execute_req(request).await {
            Ok(resp) => resp,
            Err(e) => {
                if e.is_body_write_aborted()
                    || e.is_closed()
                    || e.is_connect()
                    || e.is_canceled()
                {
                    self.sample.record_error(ValidationError::ConnectionAborted);
                    return Ok(RequestResult::Reconnect);
                } else if e.is_incomplete_message()
                    || e.is_parse()
                    || e.is_parse_too_large()
                    || e.is_parse_status()
                {
                    self.sample.record_error(ValidationError::InvalidBody(
                        Cow::Borrowed("invalid-http-body"),
                    ));
                } else if e.is_timeout() {
                    self.sample.record_error(ValidationError::Timeout);
                } else {
                    return Err(e);
                }

                return Ok(RequestResult::Ok);
            },
        };

        let elapsed_time = start.elapsed();
        let read_transfer_end = self.conn.usage().get_received_count();
        let write_transfer_end = self.conn.usage().get_written_count();

        if let Err(e) = self.validator.validate(head, body) {
            self.sample.record_error(e);
        } else {
            self.sample.record_latency(elapsed_time);
            if let Some(interval) = self.expected_interval.expected_interval_us() {
                self.sample
                    .record_latency_corrected(elapsed_time, interval);
            }
            self.expected_interval =
                self.expected_interval.record(elapsed_time.as_micros() as u64);
            self.sample.record_read_transfer(
                read_transfer_start,
                read_transfer_end,
                elapsed_time,
            );
            self.sample.record_write_transfer(
                write_transfer_start,
                write_transfer_end,
                elapsed_time,
            );
        }

        // Submit the sample if it's window interval has elapsed.
        if self.sample_factory.should_submit(self.last_sent_sample) {
            let batch_tag = self.sample.tag();
            if !self.submit_sample(batch_tag) {
                self.set_abort();
                return Ok(RequestResult::Ok);
            }
        }

        Ok(RequestResult::Ok)
    }
}
