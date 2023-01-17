#[derive(Clone, Debug)]
pub(crate) enum Command {
    // Core commands
    LookupUpstream { id: String, client: String, path: String, method: String },
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