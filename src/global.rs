use crate::structs::PrintPrefix;
use colored::Colorize;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use crate::cdn::{Hosts, Server};

pub const GH_OWNER: &str = "alterware";
pub const GH_REPO: &str = "alterware-launcher";
pub const DEFAULT_MASTER: &str = "https://cdn.alterware.ovh";

pub const CDN_HOSTS: [Server; 2] = [
    Server::new("cdn.alterware.ovh"),
    Server::new("us-cdn.alterware.ovh"),
];

pub static USER_AGENT: Lazy<String> = Lazy::new(|| {
    format!(
        "AlterWare Launcher v{} on {} | github.com/{}/{}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        GH_OWNER,
        GH_REPO
    )
});

pub static MASTER_URL: Lazy<Mutex<String>> = Lazy::new(|| Mutex::new(String::from(DEFAULT_MASTER)));

pub static IS_OFFLINE: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

pub static PREFIXES: Lazy<HashMap<&'static str, PrintPrefix>> = Lazy::new(|| {
    HashMap::from([
        (
            "info",
            PrintPrefix {
                text: "Info".bright_magenta(),
                padding: 8,
            },
        ),
        (
            "downloading",
            PrintPrefix {
                text: "Downloading".bright_yellow(),
                padding: 1,
            },
        ),
        (
            "checked",
            PrintPrefix {
                text: "Checked".bright_blue(),
                padding: 5,
            },
        ),
        (
            "removed",
            PrintPrefix {
                text: "Removed".bright_red(),
                padding: 5,
            },
        ),
        (
            "error",
            PrintPrefix {
                text: "Error".red(),
                padding: 7,
            },
        ),
        (
            "renamed",
            PrintPrefix {
                text: "Renamed".bright_blue(),
                padding: 5,
            },
        ),
    ])
});
