use crate::modules::core::context::CoreError;
use crate::modules::core::route::Route;
use crate::modules::core::upstream::UpstreamAddress;

#[derive(Clone, Debug)]
pub(crate) enum Event {
    // Core events
    UpstreamWasFound { cmd_id: String, upstream_address: UpstreamAddress },
    UpstreamWasNotFound { cmd_id: String },
    UpstreamWasEnabled { cmd_id: String, upstream_address: UpstreamAddress },
    UpstreamWasDisabled { cmd_id: String, upstream_address: UpstreamAddress },
    RouteWasAdded { cmd_id: String, route: Route },
    RouteWasNotAdded { cmd_id: String, route: Route, error: CoreError },
    RouteWasRemoved { cmd_id: String, route: Route },
    RouteWasNotRemoved { cmd_id: String, route_id: String, error: CoreError },
    RoutesWereFound { cmd_id: String, routes: Vec<Route> },
    RouteWasFound { cmd_id: String, route: Route },
    RouteWasNotFound { cmd_id: String, route_id: String },
    UpstreamsWereFound { cmd_id: String, upstreams: Vec<UpstreamAddress> },

    // Probe events
    ProbeWasStarted { cmd_id: String },
    ProbeWasStopped { cmd_id: String },

    // Stats events
    StatWasCounted { cmd_id: String },
    StatsWereFound { cmd_id: String, stats: Vec<(String, String, String, String, u64)> },
}