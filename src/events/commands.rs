pub(crate) enum CoreCommand {
    LookupUpstream { id: String },
    EnableUpstream { id: String },
    DisableUpstream { id: String },
    AddRoute { id: String },
    RemoveRoute { id: String },
}

pub(crate) enum ProbeCommand {
    StartProbe { id: String },
    PauseProbe { id: String },
    StopProbe { id: String },
}

pub(crate) enum StatsCommand {
    CountStat { id: String },
    LookupStats { id: String },
}