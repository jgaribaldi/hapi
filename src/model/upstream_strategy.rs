use crate::Route;

pub trait UpstreamStrategy {
    fn next_for(&self, route: &Route) -> Option<String>;
}

#[derive(Clone, Debug)]
pub struct AlwaysFirstUpstreamStrategy {
}

impl AlwaysFirstUpstreamStrategy {
    pub fn build() -> Self {
        AlwaysFirstUpstreamStrategy {}
    }
}

impl UpstreamStrategy for AlwaysFirstUpstreamStrategy {
    fn next_for(&self, route: &Route) -> Option<String> {
        route.upstreams.first()
            .map(|upstream| String::from(upstream))
    }
}