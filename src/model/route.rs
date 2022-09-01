use crate::model::upstream::{UpstreamAddress, UpstreamStrategy};
use crate::Upstream;

#[derive(Clone, Debug)]
pub struct Route {
    pub name: String,
    pub methods: Vec<String>,
    pub paths: Vec<String>,
    pub upstreams: Vec<Upstream>,
    pub strategy: Box<dyn UpstreamStrategy>,
}

impl Route {
    pub fn build(
        name: String,
        methods: Vec<String>,
        paths: Vec<String>,
        upstreams: Vec<Upstream>,
        strategy: Box<dyn UpstreamStrategy>,
    ) -> Self {
        Route {
            name,
            methods,
            paths,
            upstreams,
            strategy,
        }
    }

    pub fn get_upstream_by_address(&self, address: &UpstreamAddress) -> Option<&Upstream> {
        for u in self.upstreams.iter() {
            if u.address == *address {
                return Some(u);
            }
        }
        return None;
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
        let available_upstreams: Vec<&Upstream> =
            self.upstreams.iter().filter(|u| u.enabled).collect();

        if available_upstreams.len() == 0 {
            None
        } else {
            let next_upstream_index = self.strategy.next(available_upstreams.as_slice());
            Some(available_upstreams[next_upstream_index])
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::route::Route;
    use crate::model::upstream::UpstreamAddress;
    use crate::{RoundRobinUpstreamStrategy, Upstream};

    #[test]
    fn should_enable_upstream() {
        // given:
        let mut route = sample_route();
        let ups_addr = UpstreamAddress::FQDN(String::from("upstream2"));

        // when:
        route.enable_upstream(&ups_addr);

        // then:
        let u = route.get_upstream_by_address(&ups_addr).unwrap();
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
        let u = route.get_upstream_by_address(&ups_addr).unwrap();
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
        let strategy = RoundRobinUpstreamStrategy::build(0);
        let upstream1 = Upstream::build_from_fqdn("upstream1");
        let mut upstream2 = Upstream::build_from_fqdn("upstream2");
        upstream2.disable();
        let upstream3 = Upstream::build_from_fqdn("upstream3");

        Route::build(
            String::from("route1"),
            vec![String::from("GET")],
            vec![String::from("uri1"), String::from("uri2")],
            vec![upstream1, upstream2, upstream3],
            Box::new(strategy),
        )
    }
}
