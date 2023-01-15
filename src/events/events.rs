pub(crate) enum CoreEvent {
    UpstreamWasFound { cmd_id: String },
    UpstreamWasNotFound { cmd_id: String },
    UpstreamWasEnabled { cmd_id: String },
    UpstreamWasDisabled { cmd_id: String },
    RouteWasAdded { cmd_id: String },
    RouteWasNotAdded { cmd_id: String },
    RouteWasRemoved { cmd_id: String },
    RouteWasNotRemoved { cmd_id: String },
}

pub(crate) enum ProbeEvent {
    ProbeWasStarted { cmd_id: String },
    ProbeWasPaused { cmd_id: String },
    ProbeWasStopped { cmd_id: String },
}

pub(crate) enum StatsEvent {
    StatWasCounted { cmd_id: String },
    StatsWereFound { cmd_id: String },
    StatsWereNotFound { cmd_id: String },
}