use crate::modules::core::upstream::UpstreamAddress;

#[derive(Clone, Debug)]
pub(crate) enum Command {
    // Core commands
    LookupUpstream { id: String, client: String, path: String, method: String },
    EnableUpstream { id: String },
    DisableUpstream { id: String },
    AddRoute { id: String },
    RemoveRoute { id: String },

    // Probe commands
    StartProbe { id: String, upstream_address: UpstreamAddress },
    StopProbe { id: String, upstream_address: UpstreamAddress },

    // Stats commands
    CountStat { id: String },
    LookupStats { id: String },
}