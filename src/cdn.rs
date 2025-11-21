use crate::http;
use futures::future::join_all;
use simple_log::*;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

static CURRENT_CDN: Mutex<Option<Arc<Server>>> = Mutex::new(None);

#[derive(Clone, Copy, Debug)]
pub struct Server {
    pub host: &'static str,
    pub rating: u8,
    pub latency: Option<std::time::Duration>,
}

impl Server {
    pub const fn new(host: &'static str) -> Self {
        Server {
            host,
            rating: 255,
            latency: None,
        }
    }

    pub fn url(&self) -> String {
        format!("https://{}/", self.host)
    }

    async fn rate(&mut self, asn: u32, is_initial: bool) {
        let timeout = if is_initial {
            Duration::from_millis(1000)
        } else {
            Duration::from_millis(5000)
        };
        match http::rating_request(&self.url(), timeout).await {
            Ok((latency, is_cloudflare)) => {
                self.latency = Some(latency);
                self.rating = self.calculate_rating(latency, is_cloudflare, asn);

                info!(
                    "Server {} rated {} ({}ms, rating: {}, cloudflare: {})",
                    self.host,
                    self.rating,
                    latency.as_millis(),
                    self.rating,
                    is_cloudflare,
                );
            }
            Err(e) => {
                error!("Failed to connect to {}: {}", self.host, e);
                self.rating = 0;
                self.latency = None;
            }
        }
    }

    fn rate_latency(&self, latency: std::time::Duration) -> u8 {
        let ms = latency.as_millis() as f32;

        let rating = if ms <= 50.0 {
            240.0
        } else if ms <= 100.0 {
            240.0 - (ms - 50.0) * 1.0
        } else if ms <= 200.0 {
            190.0 - (ms - 100.0) * 0.5
        } else if ms <= 500.0 {
            140.0 - (ms - 200.0) * 0.033
        } else {
            100.0
        };

        rating.clamp(1.0, 255.0) as u8
    }

    fn calculate_rating(&self, latency: std::time::Duration, is_cloudflare: bool, asn: u32) -> u8 {
        let mut rating = self.rate_latency(latency);

        // Additional factors for full rating
        if is_cloudflare {
            // 3320/DTAG: bad cf peering
            // 5483/Magyar Telekom: sub. of DTAG
            if asn == 3320 || asn == 5483 {
                rating = (rating as f32 * 0.1) as u8;
            }
        }

        rating
    }
}

pub struct Hosts {
    pub servers: Vec<Server>,
    pub active_index: RwLock<Option<usize>>,
}

impl Hosts {
    /// create new rated hosts instance
    pub async fn new() -> Self {
        let cdn_hosts = crate::global::CDN_HOSTS.to_vec();

        let hosts = Hosts {
            servers: cdn_hosts,
            active_index: RwLock::new(None),
        };

        if hosts.servers.iter().all(|server| server.rating == 0) {
            info!("All CDN servers failed with 1000ms timeout, retrying with 5000ms timeout");
        }

        hosts
    }

    /// get the URL of the currently active CDN
    pub fn active_url(&self) -> Option<String> {
        CURRENT_CDN.lock().unwrap().as_ref().map(|s| s.url())
    }

    /// set the next best host based on ratings
    pub fn next(&self) -> bool {
        if self.servers.is_empty() {
            return false;
        }

        // find best host by rating, then by latency
        if let Some((idx, _)) = self.servers.iter().enumerate().max_by_key(|(_, server)| {
            (
                server.rating,
                server
                    .latency
                    .map_or(0, |l| u64::MAX - l.as_millis() as u64),
            )
        }) {
            let server = &self.servers[idx];
            *CURRENT_CDN.lock().unwrap() = Some(Arc::new(*server));
            *self.active_index.write().unwrap() = Some(idx);
            true
        } else {
            false
        }
    }

    /// rate and order all servers, then select the best one
    pub async fn rate(&mut self, asn: u32, is_initial: bool) {
        let rating_futures: Vec<_> = self
            .servers
            .iter_mut()
            .map(|server| server.rate(asn, is_initial))
            .collect();

        join_all(rating_futures).await;

        // reset state and select best host
        *self.active_index.write().unwrap() = None;
        *CURRENT_CDN.lock().unwrap() = None;
        self.next();
    }

    /// Get the best CDN URL for use
    pub fn get_master_url(&self) -> Option<String> {
        self.active_url()
    }
}
