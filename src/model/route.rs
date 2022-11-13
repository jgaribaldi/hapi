use crate::model::upstream::{Upstream, UpstreamAddress, UpstreamStrategy};

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub id: String,
    pub name: String,
    pub methods: Vec<String>,
    pub paths: Vec<String>,
    pub upstreams: Vec<Upstream>,
    pub strategy: UpstreamStrategy,
}

impl Route {
    pub fn build(
        id: String,
        name: String,
        methods: Vec<String>,
        paths: Vec<String>,
        upstreams: Vec<Upstream>,
        strategy: UpstreamStrategy,
    ) -> Self {
        Route {
            id,
            name,
            methods,
            paths,
            upstreams,
            strategy,
        }
    }

    pub fn enable_upstream(&mut self, upstream: &UpstreamAddress) {
        for u in self.upstreams.iter_mut() {
            if u.address == *upstream && !u.enabled {
                u.enable()
            }
        }
    }

    pub fn disable_upstream(&mut self, upstream: &UpstreamAddress) {
        for u in self.upstreams.iter_mut() {
            if u.address == *upstream && u.enabled {
                u.disable()
            }
        }
    }

    pub fn next_available_upstream(&mut self) -> Option<&Upstream> {
        let enabled_upstreams: Vec<&Upstream> =
            self.upstreams.iter().filter(|u| u.enabled).collect();

        if enabled_upstreams.len() > 0 {
            let next_upstream_index = self.strategy.next(enabled_upstreams.as_slice());
            Some(enabled_upstreams[next_upstream_index])
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::route::Route;
    use crate::model::upstream::{Upstream, UpstreamAddress, UpstreamStrategy};

    #[test]
    fn should_enable_upstream() {
        // given:
        let mut route = sample_route();
        let ups_addr = UpstreamAddress::FQDN(String::from("upstream2"));

        // when:
        route.enable_upstream(&ups_addr);

        // then:
        let u = get_upstream_by_address(&route, &ups_addr).unwrap();
        assert_eq!(true, u.enabled)
    }

    #[test]
    fn should_disable_upstream() {
        // given:
        let mut route = sample_route();
        let ups_addr = UpstreamAddress::FQDN(String::from("upstream1"));

        // when:
        route.disable_upstream(&ups_addr);

        // then:
        let u = get_upstream_by_address(&route, &ups_addr).unwrap();
        assert_eq!(false, u.enabled)
    }

    #[test]
    fn should_return_next_available_upstream() {
        // given:
        let mut route = sample_route();

        // when:
        let upstream1 = route.next_available_upstream().unwrap().clone();
        let upstream2 = route.next_available_upstream().unwrap().clone();
        let upstream3 = route.next_available_upstream().unwrap().clone();
        let upstream4 = route.next_available_upstream().unwrap().clone();

        // then:
        assert_eq!(String::from("upstream1"), upstream1.address.to_string());
        assert_eq!(String::from("upstream3"), upstream2.address.to_string());
        assert_eq!(String::from("upstream1"), upstream3.address.to_string());
        assert_eq!(String::from("upstream3"), upstream4.address.to_string());
    }

    fn sample_route() -> Route {
        let strategy = UpstreamStrategy::RoundRobin { index: 0 };
        let upstream1 = Upstream::build_from_fqdn("upstream1");
        let mut upstream2 = Upstream::build_from_fqdn("upstream2");
        upstream2.disable();
        let upstream3 = Upstream::build_from_fqdn("upstream3");

        Route::build(
            String::from("id1"),
            String::from("route1"),
            vec![String::from("GET")],
            vec![String::from("uri1"), String::from("uri2")],
            vec![upstream1, upstream2, upstream3],
            strategy,
        )
    }

    fn get_upstream_by_address(route: &Route, address: &UpstreamAddress) -> Option<Upstream> {
        for u in route.upstreams.iter() {
            if u.address == *address {
                return Some(u.clone());
            }
        }
        return None;
    }
}
