//! Test modules for nextcloud-connector
//!
//! This module contains all test infrastructure and test cases for the
//! nextcloud-connector crate, including mock WebDAV implementations and
//! test harnesses.

pub mod mock_webdav;
pub mod harness;
pub mod crud_tests;
pub mod async_tests;
pub mod rollback_tests;
pub mod undo_tests;
pub mod protocol_tests;
