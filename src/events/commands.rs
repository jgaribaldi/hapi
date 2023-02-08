use crate::modules::core::route::Route;
use crate::modules::core::upstream::UpstreamAddress;

#[derive(Clone, Debug)]
pub(crate) enum Command {
    // Core commands
    LookupUpstream {
        id: String,
        client: String,
        path: String,
        method: String,
    },
    EnableUpstream {
        id: String,
        upstream_address: UpstreamAddress,
    },
    DisableUpstream {
        id: String,
        upstream_address: UpstreamAddress,
    },
    AddRoute {
        id: String,
        route: Route,
    },
    RemoveRoute {
        id: String,
        route_id: String,
    },
    LookupAllRoutes {
        id: String,
    },
    LookupRoute {
        id: String,
        route_id: String,
    },
    LookupAllUpstreams {
        id: String,
    },

    // Stats commands
    LookupStats {
        id: String,
    },
}
