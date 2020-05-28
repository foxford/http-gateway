use super::super::config::TelemetryConfig;
use anyhow::Result;
use chrono::{serde::ts_seconds, DateTime, Utc};
use svc_agent::{
    mqtt::{
        IncomingEvent, IntoPublishableMessage, OutgoingEvent, ResponseStatus,
        ShortTermTimingProperties,
    },
    queue_counter::QueueCounterHandle,
    AgentId,
};
use svc_error::Error as SvcError;

#[derive(Debug, Deserialize)]
pub(crate) struct PullPayload {
    #[serde(default = "default_duration")]
    duration: u64,
}

fn default_duration() -> u64 {
    5
}

#[derive(Serialize)]
pub(crate) struct MetricValue {
    value: u64,
    #[serde(with = "ts_seconds")]
    timestamp: DateTime<Utc>,
}

#[derive(Serialize)]
#[serde(tag = "metric")]
pub(crate) enum Metric {
    #[serde(rename(serialize = "apps.http_gateway.incoming_requests_total"))]
    IncomingQueueRequests(MetricValue),
    #[serde(rename(serialize = "apps.http_gateway.incoming_responses_total"))]
    IncomingQueueResponses(MetricValue),
    #[serde(rename(serialize = "apps.http_gateway.incoming_events_total"))]
    IncomingQueueEvents(MetricValue),
    #[serde(rename(serialize = "apps.http_gateway.outgoing_requests_total"))]
    OutgoingQueueRequests(MetricValue),
    #[serde(rename(serialize = "apps.http_gateway.outgoing_responses_total"))]
    OutgoingQueueResponses(MetricValue),
    #[serde(rename(serialize = "apps.http_gateway.outgoing_events_total"))]
    OutgoingQueueEvents(MetricValue),
}

pub(crate) struct PullHandler;

impl PullHandler {
    pub(crate) fn handle(
        message: IncomingEvent<String>,
        telemetry: &TelemetryConfig,
        queue_counter: QueueCounterHandle,
        agent_id: AgentId,
    ) -> Result<Option<Box<dyn IntoPublishableMessage>>> {
        let payload: PullPayload = serde_json::from_str(message.payload())?;
        match telemetry {
            TelemetryConfig {
                id: Some(ref account_id),
            } => {
                let now = Utc::now();

                let stats = queue_counter.get_stats(payload.duration).map_err(|e| {
                    SvcError::builder()
                        .status(ResponseStatus::BAD_REQUEST)
                        .detail(e.as_ref())
                        .build()
                })?;

                let metrics = vec![
                    Metric::IncomingQueueRequests(MetricValue {
                        value: stats.incoming_requests,
                        timestamp: now,
                    }),
                    Metric::IncomingQueueResponses(MetricValue {
                        value: stats.incoming_responses,
                        timestamp: now,
                    }),
                    Metric::IncomingQueueEvents(MetricValue {
                        value: stats.incoming_events,
                        timestamp: now,
                    }),
                    Metric::OutgoingQueueRequests(MetricValue {
                        value: stats.outgoing_requests,
                        timestamp: now,
                    }),
                    Metric::OutgoingQueueResponses(MetricValue {
                        value: stats.outgoing_responses,
                        timestamp: now,
                    }),
                    Metric::OutgoingQueueEvents(MetricValue {
                        value: stats.outgoing_events,
                        timestamp: now,
                    }),
                ];

                let short_term_timing = ShortTermTimingProperties::until_now(Utc::now());
                let mut props = message
                    .properties()
                    .to_event("metric.create", short_term_timing);
                props.set_agent_id(agent_id);

                let outgoing_event = OutgoingEvent::multicast(metrics, props, account_id);
                let boxed_event =
                    Box::new(outgoing_event) as Box<dyn IntoPublishableMessage + Send>;
                Ok(Some(boxed_event))
            }

            _ => Ok(None),
        }
    }
}
