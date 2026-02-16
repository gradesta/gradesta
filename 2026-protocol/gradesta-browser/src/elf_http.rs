//! HTTP client for elf communication
//!
//! This module handles HTTP requests to elf services:
//! - Fetching manifests from elf URLs
//! - POSTing summon requests to elves

use std::thread;

use crossbeam_channel::Sender;

use crate::state::{ElfCommand, ElfInputType, ElfManifest};

/// Events that can result from elf HTTP operations
#[derive(Debug)]
pub enum ElfHttpEvent {
    /// Manifest fetched successfully
    ManifestFetched {
        elf_url: String,
        manifest: ElfManifest,
    },
    /// Failed to fetch manifest
    ManifestFetchFailed {
        elf_url: String,
        error: String,
    },
    /// Summon request accepted
    SummonAccepted {
        elf_url: String,
    },
    /// Summon request failed
    SummonFailed {
        elf_url: String,
        error: String,
    },
}

/// Fetch an elf manifest in the background
pub fn fetch_manifest_async(elf_url: String, tx: Sender<ElfHttpEvent>) {
    thread::spawn(move || {
        let manifest_url = format!("{}/manifest.json", elf_url.trim_end_matches('/'));
        eprintln!("Fetching elf manifest from: {}", manifest_url);

        match reqwest::blocking::get(&manifest_url) {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<ManifestJson>() {
                        Ok(json) => {
                            let manifest = ElfManifest {
                                elf_id: json.elf_id,
                                name: json.name,
                                description: json.description,
                                commands: json.commands.into_iter().map(|c| ElfCommand {
                                    name: c.name,
                                    description: c.description,
                                    inputs: c.inputs.into_iter().map(|i| match i.as_str() {
                                        "cursor" => ElfInputType::Cursor,
                                        "region" => ElfInputType::Region,
                                        "prompt" => ElfInputType::Prompt,
                                        _ => ElfInputType::Cursor,
                                    }).collect(),
                                }).collect(),
                            };
                            let _ = tx.send(ElfHttpEvent::ManifestFetched {
                                elf_url,
                                manifest,
                            });
                        }
                        Err(e) => {
                            let _ = tx.send(ElfHttpEvent::ManifestFetchFailed {
                                elf_url,
                                error: format!("Invalid manifest JSON: {}", e),
                            });
                        }
                    }
                } else {
                    let _ = tx.send(ElfHttpEvent::ManifestFetchFailed {
                        elf_url,
                        error: format!("HTTP {}", response.status()),
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(ElfHttpEvent::ManifestFetchFailed {
                    elf_url,
                    error: format!("Request failed: {}", e),
                });
            }
        }
    });
}

/// JSON structure for manifest parsing
#[derive(serde::Deserialize)]
struct ManifestJson {
    elf_id: String,
    name: String,
    description: String,
    commands: Vec<CommandJson>,
}

#[derive(serde::Deserialize)]
struct CommandJson {
    name: String,
    description: String,
    inputs: Vec<String>,
}

/// POST a summon request to an elf
pub fn summon_elf_async(
    elf_url: String,
    token: String,
    server_ws_url: String,
    command: String,
    params: std::collections::HashMap<String, String>,
    tx: Sender<ElfHttpEvent>,
) {
    thread::spawn(move || {
        let summon_url = format!("{}/summon", elf_url.trim_end_matches('/'));
        eprintln!("Summoning elf at: {}", summon_url);

        let body = serde_json::json!({
            "token": token,
            "server_ws_url": server_ws_url,
            "command": command,
            "params": params,
        });

        match reqwest::blocking::Client::new()
            .post(&summon_url)
            .json(&body)
            .send()
        {
            Ok(response) => {
                if response.status().is_success() {
                    eprintln!("Elf summon accepted");
                    let _ = tx.send(ElfHttpEvent::SummonAccepted {
                        elf_url,
                    });
                } else {
                    let _ = tx.send(ElfHttpEvent::SummonFailed {
                        elf_url,
                        error: format!("HTTP {}", response.status()),
                    });
                }
            }
            Err(e) => {
                let _ = tx.send(ElfHttpEvent::SummonFailed {
                    elf_url,
                    error: format!("Request failed: {}", e),
                });
            }
        }
    });
}
