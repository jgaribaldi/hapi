use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub async fn count_request(
    stats: Arc<Mutex<Stats>>,
    client: &str,
    method: &str,
    path: &str,
    upstream: &str,
) {
    let mut sts = stats.lock().unwrap();
    sts.count_request(client, method, path, upstream)
}

pub struct Stats {
    // (client, method, path, upstream) => count
    counter: HashMap<(String, String, String, String), u64>,
}

impl Stats {
    pub fn build() -> Self {
        Stats {
            counter: HashMap::new(),
        }
    }

    pub fn count_request(
        &mut self,
        client: &str,
        method: &str,
        path: &str,
        upstream: &str,
    ) {
        let key = (
            client.to_string(),
            method.to_string(),
            path.to_string(),
            upstream.to_string()
        );
        *self.counter.entry(key).or_insert(0) += 1;
    }

    pub fn requests_by_client(
        &self,
        client: &str,
    ) -> Option<u64> {
        self.counter.iter()
            .filter(|entry| {
                // entries matching given client
                let (cli, _, _, _) = entry.0;
                *cli == client.to_string()
            })
            .map(|entry| entry.1.clone())
            .reduce(|mut accum, value| {
                accum += value;
                return accum
            })
    }

    pub fn requests_by_upstream(
        &self,
        upstream: &str,
    ) -> Option<u64> {
        self.counter.iter()
            .filter(|entry| {
                // entries matching given upstream
                let (_, _, _, ups) = entry.0;
                *ups == upstream.to_string()
            })
            .map(|entry| entry.1.clone())
            .reduce(|mut accum, value| {
                accum += value;
                return accum
            })
    }

    pub fn print_stats(&self) {
        for entry in self.counter.iter() {
            println!("({:?}) => {}", entry.0, entry.1)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::stats::Stats;

    #[test]
    fn should_get_requests_by_client() {
        // given:
        let stats = sample_stats();

        // when:
        let result = stats.requests_by_client("client1").unwrap();

        // then:
        assert_eq!(2, result)
    }

    #[test]
    fn should_get_requests_by_upstream() {
        // given:
        let stats = sample_stats();

        // when:
        let result = stats.requests_by_upstream("upstream2").unwrap();

        // then:
        assert_eq!(3, result)
    }

    #[test]
    fn should_not_find_request_by_non_existent_client() {
        // given:
        let stats = sample_stats();

        // when:
        let result = stats.requests_by_client("client4");

        // then:
        assert_eq!(None, result)
    }

    #[test]
    fn should_not_find_request_by_non_existent_upstream() {
        // given:
        let stats = sample_stats();

        // when:
        let result = stats.requests_by_upstream("upstream4");

        // then:
        assert_eq!(None, result)
    }

    fn sample_stats() -> Stats {
        let mut stats = Stats::build();
        stats.count_request("client1", "GET", "/path", "upstream1");
        stats.count_request("client2", "GET", "/path", "upstream1");
        stats.count_request("client1", "GET", "/path2", "upstream2");
        stats.count_request("client2", "GET", "/path2", "upstream2");
        stats.count_request("client3", "GET", "/path2", "upstream2");
        stats
    }
}
