pub(crate) enum Command {
    // Core commands
    LookupUpstream { id: String },
    EnableUpstream { id: String },
    DisableUpstream { id: String },
    AddRoute { id: String },
    RemoveRoute { id: String },

    // Probe commands
    StartProbe { id: String },
    StopProbe { id: String },

    // Stats commands
    CountStat { id: String },
    LookupStats { id: String },
}