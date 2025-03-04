use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use substrate_prometheus_endpoint::{
    register, Counter, MetricSource, Opts, PrometheusError, Registry, SourcedGauge, U64,
};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Event {
    Broadcast,
    SendRequest,
    SendTo,
    SendExtensionRequest,
    HandleState,
    HandleRequestResponse,
    HandleRequest,
    HandleExtensionRequest,
    HandleTask,
    HandleBlockImported,
    HandleBlockFinalized,
    HandleStateResponse,
    HandleJustificationFromUser,
    HandleInternalRequest,
}

use Event::*;

impl Event {
    fn name(&self) -> &str {
        match self {
            Broadcast => "broadcast",
            SendRequest => "send_request",
            SendTo => "send_to",
            SendExtensionRequest => "send_extension_request",
            HandleState => "handle_state",
            HandleRequestResponse => "handle_request_response",
            HandleRequest => "handle_request",
            HandleExtensionRequest => "handle_extension_request",
            HandleTask => "handle_task",
            HandleBlockImported => "handle_block_imported",
            HandleBlockFinalized => "handle_block_finalized",
            HandleStateResponse => "handle_state_response",
            HandleJustificationFromUser => "handle_justification_from_user",
            HandleInternalRequest => "handle_internal_request",
        }
    }
}

const ALL_EVENTS: [Event; 14] = [
    Broadcast,
    SendRequest,
    SendTo,
    SendExtensionRequest,
    HandleState,
    HandleRequestResponse,
    HandleRequest,
    HandleExtensionRequest,
    HandleTask,
    HandleBlockImported,
    HandleBlockFinalized,
    HandleStateResponse,
    HandleJustificationFromUser,
    HandleInternalRequest,
];

const ERRORING_EVENTS: [Event; 11] = [
    Broadcast,
    SendRequest,
    SendTo,
    SendExtensionRequest,
    HandleState,
    HandleRequest,
    HandleExtensionRequest,
    HandleTask,
    HandleBlockImported,
    HandleJustificationFromUser,
    HandleInternalRequest,
];

pub enum Metrics {
    Prometheus {
        event_calls: HashMap<Event, Counter<U64>>,
        event_errors: HashMap<Event, Counter<U64>>,
    },
    Noop,
}

#[derive(Clone)]
pub struct MajorSyncingGauge(Arc<AtomicBool>);

impl MajorSyncingGauge {
    fn register(registry: &Registry, value: Arc<AtomicBool>) -> Result<(), PrometheusError> {
        register(
            SourcedGauge::new(
                &Opts::new(
                    "aleph_sync_is_major_syncing",
                    "Whether the node is performing a major sync or not.",
                ),
                MajorSyncingGauge(value),
            )?,
            registry,
        )?;

        Ok(())
    }
}

impl MetricSource for MajorSyncingGauge {
    type N = u64;

    fn collect(&self, mut set: impl FnMut(&[&str], Self::N)) {
        set(&[], self.0.load(Ordering::Relaxed) as u64);
    }
}

impl Metrics {
    pub fn new(
        is_major_syncing: Arc<AtomicBool>,
        registry: Option<Registry>,
    ) -> Result<Self, PrometheusError> {
        let registry = match registry {
            Some(registry) => registry,
            None => return Ok(Metrics::Noop),
        };
        let mut event_calls = HashMap::new();
        let mut event_errors = HashMap::new();
        for event in ALL_EVENTS {
            event_calls.insert(
                event,
                register(
                    Counter::new(
                        format!("aleph_sync_{}", event.name()),
                        format!("number of times {} has been called", event.name()),
                    )?,
                    &registry,
                )?,
            );
        }
        for event in ERRORING_EVENTS {
            event_errors.insert(
                event,
                register(
                    Counter::new(
                        format!("aleph_sync_{}_error", event.name()),
                        format!("number of times {} has returned an error", event.name()),
                    )?,
                    &registry,
                )?,
            );
        }

        MajorSyncingGauge::register(&registry, is_major_syncing)?;

        Ok(Metrics::Prometheus {
            event_calls,
            event_errors,
        })
    }

    pub fn noop() -> Self {
        Metrics::Noop
    }

    pub fn report_event(&self, event: Event) {
        if let Metrics::Prometheus { event_calls, .. } = self {
            if let Some(counter) = event_calls.get(&event) {
                counter.inc();
            }
        }
    }

    pub fn report_event_error(&self, event: Event) {
        if let Metrics::Prometheus { event_errors, .. } = self {
            if let Some(counter) = event_errors.get(&event) {
                counter.inc();
            }
        }
    }
}
