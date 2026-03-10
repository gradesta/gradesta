//! Authentication handlers

use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use anyhow::{anyhow, Result};
use axum::extract::ws::Message as AxumWsMessage;
use futures_util::SinkExt;

use crate::connection_manager::SharedConnectionManager;
use crate::elf::SharedElfRegistry;
use crate::git_undo;
use crate::identity;
use crate::migration;
use crate::nextcloud::NextcloudClient;
use crate::notes::NotesIndex;
use crate::protocol::*;
use crate::router;
use crate::state::{ConnState, ConnectionState};
use crate::storage::{Credential, CredentialStore};
use crate::sync_worker;
use crate::utils::hash_string;
use crate::webdav_mount;

/// Handle INTRODUCE_ELF message from browser
pub async fn handle_introduce_elf<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    elf_registry: &SharedElfRegistry,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, elf_url, command, _cursor_landmark, _cursor_vertex, region, permissions, params) =
        parse_introduce_elf(data)?;

    log::info!("IntroduceElf: action={} elf_url={} command={} region={:?}",
        action_id, elf_url, command, region.origin_landmark);

    // Verify the user is authenticated
    let (identity, conn_id) = {
        let s = state.lock().await;
        if s.conn_state != ConnectionState::Browsing {
            let msg = encode_log_message(action_id, 401, 0, "Not authenticated");
            write.send(AxumWsMessage::Binary(msg)).await.map_err(|e| anyhow!("{:?}", e))?;
            return Ok(());
        }
        (s.identity.clone().unwrap_or_default(), s.conn_id)
    };

    // Create invitation in registry
    let token = {
        let mut registry = elf_registry.lock().await;
        registry.create_invitation(
            identity,
            elf_url.clone(),
            region,
            permissions,
            command.clone(),
            params,
            conn_id,
            300, // 5 minute TTL
        )
    };

    log::info!("Created elf invitation with token: {}...", &token[..std::cmp::min(8, token.len())]);

    // Send IntroductionToken back to browser
    let token_msg = encode_introduction_token(action_id, &token);
    write.send(AxumWsMessage::Binary(token_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

    log::info!("Sent IntroductionToken to browser");
    Ok(())
}

pub async fn handle_identification_response<W>(
    data: &[u8],
    state: &Arc<Mutex<ConnState>>,
    cred_store: &Arc<Mutex<CredentialStore>>,
    connection_manager: &SharedConnectionManager,
    write: &mut W,
) -> Result<()>
where
    W: SinkExt<AxumWsMessage> + Unpin,
    W::Error: std::fmt::Debug,
{
    let (action_id, identity_url, signature) = parse_identification_response(data)?;

    let pending = {
        let mut s = state.lock().await;
        s.pending_auth.take()
    };

    let pending = pending.ok_or_else(|| anyhow!("No pending auth"))?;

    // Verify identity
    let verified = identity::verify_identification(&pending, action_id, &identity_url, &signature).await?;

    log::info!("Verified identity: {} ({})", verified.identity_url, verified.embedded_claim);

    // Use the identity URL as the canonical identity for storage
    let identity = verified.identity_url.clone();

    // Check for stored credentials
    let cred = {
        let store = cred_store.lock().await;
        store.get(&identity).cloned()
    };

    if let Some(cred) = cred {
        log::info!("Found stored credentials for {}", identity);

        let nc = NextcloudClient::new(&cred.nextcloud_url, &cred.username, &cred.app_password);

        // Load notes index
        let mut index = NotesIndex::load(&nc).await?;
        let mut shared_index = Arc::new(Mutex::new(index.clone()));

        // Try to mount WebDAV for efficient git operations
        let webdav_mount = match webdav_mount::WebDavMount::mount(
            &cred.nextcloud_url,
            &cred.username,
            &cred.app_password,
        ) {
            Ok(mount) => {
                log::info!("WebDAV mounted at {}", mount.path().display());
                Some(mount)
            }
            Err(e) => {
                log::warn!("Failed to mount WebDAV (falling back to direct upload): {}", e);
                None
            }
        };

        // Initialize git-based undo repo
        // If WebDAV is mounted, use git push/pull to bare repo on mount
        // Otherwise fall back to manual .git directory upload
        let git_repo = if let Some(ref mount) = webdav_mount {
            // Use mounted WebDAV with git push/pull
            let local_path = std::path::PathBuf::from(format!(
                "/tmp/gradesta-undo-{}",
                &hash_string(&format!("{}:{}", cred.nextcloud_url, cred.username)).to_string()[..8]
            ));
            match git_undo::GitUndoRepo::open_with_remote(&local_path, &mount.bare_repo_path()) {
                Ok(repo) => {
                    log::info!("Git undo repo ready with remote at {}", mount.bare_repo_path().display());
                    Some(std::sync::Arc::new(tokio::sync::Mutex::new(repo)))
                }
                Err(e) => {
                    log::warn!("Failed to initialize git undo repo with remote: {}", e);
                    None
                }
            }
        } else {
            // Fall back to old behavior: download/upload .git directory via WebDAV
            match git_undo::open_from_nextcloud(&nc).await {
                Ok(repo) => {
                    log::info!("Git undo repo ready at {}", repo.local_path().display());
                    Some(std::sync::Arc::new(tokio::sync::Mutex::new(repo)))
                }
                Err(e) => {
                    log::warn!("Failed to initialize git undo repo: {}", e);
                    None
                }
            }
        };

        // Check if migration is needed and run it automatically
        if migration::needs_migration(&index) {
            if let Some(ref git_repo) = git_repo {
                // Get workdir path from git repo (quick lock, no await)
                let workdir = {
                    let git_guard = git_repo.lock().await;
                    git_guard.workdir().map(|p| p.to_path_buf())
                };

                if let Some(workdir) = workdir {
                    log::info!("Old index format detected, running automatic migration to CAS");
                    // auto_migrate takes owned types, safe to await without holding git mutex
                    match migration::auto_migrate(workdir, nc.clone()).await {
                        Ok(_stats) => {
                            log::info!("Migration complete, reloading index");
                            // Reload the index after migration
                            match NotesIndex::load(&nc).await {
                                Ok(new_index) => {
                                    index = new_index.clone();
                                    shared_index = Arc::new(Mutex::new(new_index));
                                }
                                Err(e) => {
                                    log::error!("Failed to reload index after migration: {}", e);
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("Migration failed: {}", e);
                            // Continue with old index - it should still work
                        }
                    }
                } else {
                    log::warn!("Migration needed but git repo has no workdir");
                }
            } else {
                log::warn!("Migration needed but no git repo available");
            }
        }

        // Spawn sync worker if we have git repo
        // Get conn_id for the sync worker
        let worker_conn_id = {
            let s = state.lock().await;
            s.conn_id
        };
        let sync_tx = if let Some(ref git_repo) = git_repo {
            Some(sync_worker::SyncWorker::spawn(
                nc.clone(),
                Arc::clone(git_repo),
                Arc::clone(&shared_index),
                Arc::clone(connection_manager),
                worker_conn_id,
                identity.clone(),
            ))
        } else {
            None
        };

        // Get a server-generated action_id for the router
        let action_id = {
            let mut s = state.lock().await;
            s.identity = Some(identity.clone());
            s.nextcloud = Some(nc);
            s.index = Some(index);
            s.shared_index = Some(shared_index);
            s.git_undo_repo = git_repo;
            s.sync_tx = sync_tx;
            s.webdav_mount = webdav_mount;
            s.conn_state = ConnectionState::Browsing;
            s.get_next_action_id()
        };

        // Send router (entry point with notes and calendar branches)
        // Note: Sync worker sends responses through the connection manager's forward channel
        router::send_router(&identity, write, action_id).await?;
    } else {
        log::info!("No credentials for {}, starting auth flow", identity);

        // Extract Nextcloud URL from embedded claim (username@server)
        let parts: Vec<&str> = verified.embedded_claim.splitn(2, '@').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid embedded claim format: {}", verified.embedded_claim));
        }
        let nc_url = format!("https://{}", parts[1]);

        // Initiate login flow
        let login_flow = crate::nextcloud::initiate_login_flow(&nc_url).await?;
        log::info!("Login flow initiated: {}", login_flow.login);

        {
            let mut s = state.lock().await;
            s.identity = Some(identity.clone());
            s.conn_state = ConnectionState::AwaitingAuth;
            s.poll_endpoint = Some(login_flow.poll.endpoint);
            s.poll_token = Some(login_flow.poll.token);
        }

        // Send auth context
        let landmark = format!("nextcloud://{}/auth", identity);
        let ctx_msg = encode_set_context(0, &landmark);
        write.send(AxumWsMessage::Binary(ctx_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send login URL
        let url_vertex_id = hash_string(&format!("auth:{}", identity));
        let label_msg = encode_set_vertex_label(0, url_vertex_id, "text/x-url", login_flow.login.as_bytes());
        write.send(AxumWsMessage::Binary(label_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send instruction
        let instr_id = hash_string(&format!("instr:{}", identity));
        let instruction = format!(
            "Click the link below to authorize this server to access your Nextcloud notes.\n\nIdentity: {}",
            identity
        );
        let instr_msg = encode_set_vertex_label(0, instr_id, "text/plain", instruction.as_bytes());
        write.send(AxumWsMessage::Binary(instr_msg)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Send edges
        let edges1 = encode_set_edges(0, instr_id, 0, 0, 0, url_vertex_id, 0, 0, 0);
        let edges2 = encode_set_edges(0, url_vertex_id, 0, 0, instr_id, 0, 0, 0, 0);
        write.send(AxumWsMessage::Binary(edges1)).await.map_err(|e| anyhow!("{:?}", e))?;
        write.send(AxumWsMessage::Binary(edges2)).await.map_err(|e| anyhow!("{:?}", e))?;

        // Start polling in background
        let state_clone = Arc::clone(state);
        let cred_store_clone = Arc::clone(cred_store);
        tokio::spawn(async move {
            poll_for_auth(state_clone, cred_store_clone).await;
        });
    }

    Ok(())
}

pub async fn poll_for_auth(state: Arc<Mutex<ConnState>>, cred_store: Arc<Mutex<CredentialStore>>) {
    let mut ticker = interval(Duration::from_secs(2));

    loop {
        ticker.tick().await;

        let (endpoint, token, identity) = {
            let s = state.lock().await;
            if s.conn_state != ConnectionState::AwaitingAuth {
                return;
            }
            (
                s.poll_endpoint.clone(),
                s.poll_token.clone(),
                s.identity.clone(),
            )
        };

        let (endpoint, token, identity) = match (endpoint, token, identity) {
            (Some(e), Some(t), Some(i)) => (e, t, i),
            _ => return,
        };

        match crate::nextcloud::poll_login_completion(&endpoint, &token).await {
            Ok(Some(result)) => {
                log::info!("Auth complete for {} (username: {})", identity, result.login_name);

                let nc = NextcloudClient::new(&result.server, &result.login_name, &result.app_password);

                // Store credentials
                {
                    let mut store = cred_store.lock().await;
                    store.set(
                        &identity,
                        Credential {
                            nextcloud_url: result.server.clone(),
                            username: result.login_name.clone(),
                            app_password: result.app_password.clone(),
                        },
                    );
                    if let Err(e) = store.save() {
                        log::error!("Failed to save credentials: {}", e);
                    }
                }

                // Load notes index
                let index = match NotesIndex::load(&nc).await {
                    Ok(idx) => idx,
                    Err(e) => {
                        log::error!("Failed to load notes index: {}", e);
                        return;
                    }
                };

                {
                    let mut s = state.lock().await;
                    s.nextcloud = Some(nc);
                    s.index = Some(index);
                    s.conn_state = ConnectionState::Browsing;
                    s.poll_endpoint = None;
                    s.poll_token = None;
                }

                log::info!("Auth complete, ready to browse notes");
                // Note: We can't send messages from here since we don't have write access
                // The client should send a WatchLandmark to get the notes listing
                return;
            }
            Ok(None) => {
                // Still waiting
            }
            Err(e) => {
                log::error!("Poll error: {}", e);
            }
        }
    }
}
