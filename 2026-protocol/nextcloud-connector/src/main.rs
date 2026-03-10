//! Nextcloud Connector - stores notes, calendar, and files on Nextcloud via WebDAV/CalDAV

mod calendar;
mod connection_manager;
mod content_cache;
mod content_store;
mod elf;
mod files;
mod garbage_collect;
mod migration;
mod git_undo;
mod handlers;
mod http_stream;
mod identity;
mod local_storage;
mod nextcloud;
mod notes;
mod protocol;
mod router;
mod server;
mod state;
mod storage;
mod sync_worker;
mod utils;
mod webdav;
mod webdav_mount;

#[cfg(test)]
mod tests;

pub use server::main;
