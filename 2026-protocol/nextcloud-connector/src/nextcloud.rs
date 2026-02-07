//! Nextcloud Login Flow v2, WebDAV, and CalDAV client

use anyhow::{anyhow, Context, Result};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

/// Login Flow v2 initial response
#[derive(Debug, Deserialize)]
pub struct LoginFlowInit {
    pub poll: LoginFlowPoll,
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginFlowPoll {
    pub token: String,
    pub endpoint: String,
}

/// Login Flow v2 completion response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginFlowResult {
    pub server: String,
    pub login_name: String,
    pub app_password: String,
}

/// Nextcloud client for WebDAV operations
#[derive(Clone)]
pub struct NextcloudClient {
    pub url: String,
    /// Base URL for the Nextcloud server (same as url, exposed for http_stream)
    pub base_url: String,
    pub username: String,
    pub password: String,
    client: Client,
}

impl NextcloudClient {
    pub fn new(url: &str, username: &str, password: &str) -> Self {
        let base = url.trim_end_matches('/').to_string();
        Self {
            url: base.clone(),
            base_url: base,
            username: username.to_string(),
            password: password.to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
        }
    }

    /// WebDAV base URL
    fn webdav_url(&self, path: &str) -> String {
        format!(
            "{}/remote.php/dav/files/{}/{}",
            self.url,
            self.username,
            path.trim_start_matches('/')
        )
    }

    /// Upload a file via WebDAV PUT
    pub async fn upload(&self, path: &str, content: &[u8]) -> Result<()> {
        // First, create parent directory if needed
        if let Some(parent) = path.rsplit_once('/').map(|(p, _)| p) {
            if !parent.is_empty() {
                let _ = self.mkdir(parent).await; // Ignore error if exists
            }
        }

        let url = self.webdav_url(path);
        let response = self
            .client
            .put(&url)
            .basic_auth(&self.username, Some(&self.password))
            .body(content.to_vec())
            .send()
            .await
            .context("WebDAV PUT failed")?;

        if !response.status().is_success() && response.status().as_u16() != 201 && response.status().as_u16() != 204 {
            return Err(anyhow!("WebDAV upload failed: {}", response.status()));
        }

        Ok(())
    }

    /// Download a file via WebDAV GET
    pub async fn download(&self, path: &str) -> Result<Vec<u8>> {
        let url = self.webdav_url(path);
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("WebDAV GET failed")?;

        if !response.status().is_success() {
            return Err(anyhow!("WebDAV download failed: {}", response.status()));
        }

        Ok(response.bytes().await?.to_vec())
    }

    /// Download a file and return content with mime type
    pub async fn download_with_type(&self, path: &str, max_size: u64) -> Result<(Vec<u8>, String)> {
        let url = self.webdav_url(path);
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("WebDAV GET failed")?;

        if !response.status().is_success() {
            return Err(anyhow!("WebDAV download failed: {}", response.status()));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        // Read up to max_size bytes
        let bytes = response.bytes().await?;
        let content = if bytes.len() as u64 > max_size {
            bytes[..max_size as usize].to_vec()
        } else {
            bytes.to_vec()
        };

        Ok((content, content_type))
    }

    /// Fetch thumbnail for a file from Nextcloud preview API
    /// Returns None if no preview is available (404)
    pub async fn get_thumbnail(&self, path: &str, width: u32, height: u32) -> Result<Option<(Vec<u8>, String)>> {
        let url = format!(
            "{}/index.php/apps/files/api/v1/thumbnail/{}/{}/{}",
            self.url, width, height, path.trim_start_matches('/')
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("Thumbnail fetch failed")?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None); // No preview available
        }

        if !response.status().is_success() {
            return Err(anyhow!("Thumbnail fetch failed: {}", response.status()));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();

        Ok(Some((response.bytes().await?.to_vec(), content_type)))
    }

    /// Create a directory via WebDAV MKCOL
    pub async fn mkdir(&self, path: &str) -> Result<()> {
        let url = self.webdav_url(path);
        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("WebDAV MKCOL failed")?;

        // 201 = created, 405 = already exists
        if !response.status().is_success() && response.status().as_u16() != 405 {
            return Err(anyhow!("WebDAV mkdir failed: {}", response.status()));
        }

        Ok(())
    }

    /// Delete a file or directory via WebDAV DELETE
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = self.webdav_url(path);
        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("WebDAV DELETE failed")?;

        if !response.status().is_success() && response.status().as_u16() != 404 {
            return Err(anyhow!("WebDAV delete failed: {}", response.status()));
        }

        Ok(())
    }

    /// Check if a file exists
    pub async fn exists(&self, path: &str) -> bool {
        let url = self.webdav_url(path);
        if let Ok(response) = self
            .client
            .head(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
        {
            response.status().is_success()
        } else {
            false
        }
    }

    /// List directory contents via WebDAV PROPFIND
    pub async fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>> {
        let url = self.webdav_url(path);
        log::info!("WebDAV: Listing directory {}", url);

        let propfind_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:oc="http://owncloud.org/ns" xmlns:nc="http://nextcloud.org/ns">
  <d:prop>
    <d:displayname/>
    <d:getcontenttype/>
    <d:getcontentlength/>
    <d:getlastmodified/>
    <d:resourcetype/>
    <oc:size/>
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(propfind_body)
            .send()
            .await
            .context("WebDAV PROPFIND failed")?;

        if !response.status().is_success() && response.status().as_u16() != 207 {
            return Err(anyhow!("WebDAV list directory failed: {}", response.status()));
        }

        let body = response.text().await?;
        log::debug!("WebDAV: PROPFIND response:\n{}", body);
        parse_file_list(&body, path)
    }

    // ========== CalDAV Methods ==========

    /// CalDAV base URL for calendars
    fn caldav_url(&self) -> String {
        format!(
            "{}/remote.php/dav/calendars/{}/",
            self.url,
            self.username
        )
    }

    /// List all calendars for the user
    pub async fn list_calendars(&self) -> Result<Vec<CalendarInfo>> {
        let url = self.caldav_url();
        log::info!("CalDAV: Listing calendars from {}", url);

        let propfind_body = r#"<?xml version="1.0" encoding="UTF-8"?>
<d:propfind xmlns:d="DAV:" xmlns:cs="http://calendarserver.org/ns/" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:displayname/>
    <cs:getctag/>
    <d:resourcetype/>
    <c:supported-calendar-component-set/>
  </d:prop>
</d:propfind>"#;

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(propfind_body)
            .send()
            .await
            .context("CalDAV PROPFIND failed")?;

        log::info!("CalDAV: PROPFIND response status: {}", response.status());

        if !response.status().is_success() && response.status().as_u16() != 207 {
            return Err(anyhow!("CalDAV list calendars failed: {}", response.status()));
        }

        let body = response.text().await?;
        log::debug!("CalDAV: PROPFIND response body:\n{}", body);
        let calendars = parse_calendar_list(&body, &self.caldav_url())?;
        log::info!("CalDAV: Found {} calendars: {:?}", calendars.len(), calendars.iter().map(|c| &c.name).collect::<Vec<_>>());
        Ok(calendars)
    }

    /// Fetch events from a calendar within a date range
    pub async fn fetch_events(
        &self,
        calendar_path: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<CalendarEvent>> {
        let url = format!("{}{}", self.caldav_url(), calendar_path.trim_matches('/'));
        log::info!("CalDAV: Fetching events from {} for {} to {}", url, start, end);

        // Format dates for CalDAV query (YYYYMMDD format)
        let start_str = start.format("%Y%m%dT000000Z").to_string();
        let end_str = end.format("%Y%m%dT235959Z").to_string();

        let report_body = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag/>
    <c:calendar-data/>
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        <c:time-range start="{}" end="{}"/>
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>"#, start_str, end_str);

        log::debug!("CalDAV: REPORT request body:\n{}", report_body);

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"REPORT").unwrap(), &url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Depth", "1")
            .header("Content-Type", "application/xml; charset=utf-8")
            .body(report_body)
            .send()
            .await
            .context("CalDAV REPORT failed")?;

        log::info!("CalDAV: REPORT response status: {}", response.status());

        if !response.status().is_success() && response.status().as_u16() != 207 {
            return Err(anyhow!("CalDAV fetch events failed: {}", response.status()));
        }

        let body = response.text().await?;
        log::debug!("CalDAV: REPORT response body:\n{}", body);
        let events = parse_events_response(&body)?;
        log::info!("CalDAV: Parsed {} events from calendar {}", events.len(), calendar_path);
        Ok(events)
    }

    /// Fetch all events for a specific day
    pub async fn fetch_events_for_day(
        &self,
        calendar_path: &str,
        date: NaiveDate,
    ) -> Result<Vec<CalendarEvent>> {
        // Fetch events for the single day
        self.fetch_events(calendar_path, date, date).await
    }

    /// Fetch events for a day from all calendars
    pub async fn fetch_all_events_for_day(&self, date: NaiveDate) -> Result<Vec<CalendarEvent>> {
        log::info!("CalDAV: Fetching all events for day {}", date);
        let calendars = self.list_calendars().await?;
        let mut all_events = Vec::new();

        for cal in &calendars {
            log::info!("CalDAV: Calendar '{}' path='{}' supports_events={}", cal.name, cal.path, cal.supports_events);
            if cal.supports_events {
                match self.fetch_events_for_day(&cal.path, date).await {
                    Ok(events) => {
                        log::info!("CalDAV: Got {} events from calendar '{}'", events.len(), cal.name);
                        all_events.extend(events);
                    }
                    Err(e) => log::warn!("Failed to fetch events from {}: {}", cal.name, e),
                }
            }
        }

        // Sort events by start time
        all_events.sort_by(|a, b| a.start.cmp(&b.start));
        log::info!("CalDAV: Total {} events for day {}", all_events.len(), date);
        Ok(all_events)
    }

    // ========== HTTP Streaming Methods ==========

    /// Get file size via WebDAV HEAD request
    pub async fn get_file_size(&self, path: &str) -> Result<u64> {
        let url = self.webdav_url(path);
        let response = self
            .client
            .head(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("WebDAV HEAD failed")?;

        if !response.status().is_success() {
            return Err(anyhow!("WebDAV HEAD failed: {}", response.status()));
        }

        let size = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        Ok(size)
    }

    /// Stream file content with Range support
    /// Returns a stream of bytes for the requested range
    pub async fn stream_file(
        &self,
        path: &str,
        start: u64,
        end: u64,
    ) -> Result<impl futures_util::Stream<Item = Result<bytes::Bytes, reqwest::Error>>> {
        let url = self.webdav_url(path);
        let range = format!("bytes={}-{}", start, end);

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.username, Some(&self.password))
            .header("Range", range)
            .send()
            .await
            .context("WebDAV GET (streaming) failed")?;

        if !response.status().is_success() && response.status().as_u16() != 206 {
            return Err(anyhow!("WebDAV streaming failed: {}", response.status()));
        }

        Ok(response.bytes_stream())
    }

    /// Get file info (size and content type) via WebDAV HEAD request
    pub async fn get_file_info(&self, path: &str) -> Result<(u64, String)> {
        let url = self.webdav_url(path);
        let response = self
            .client
            .head(&url)
            .basic_auth(&self.username, Some(&self.password))
            .send()
            .await
            .context("WebDAV HEAD failed")?;

        if !response.status().is_success() {
            return Err(anyhow!("WebDAV HEAD failed: {}", response.status()));
        }

        let size = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        Ok((size, content_type))
    }
}

/// Information about a file or directory
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: u64,
    pub content_type: Option<String>,
}

/// Information about a calendar
#[derive(Debug, Clone)]
pub struct CalendarInfo {
    pub name: String,
    pub path: String,
    pub supports_events: bool,
}

/// A calendar event
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub uid: String,
    pub summary: String,
    pub description: Option<String>,
    pub start: NaiveDateTime,
    pub end: Option<NaiveDateTime>,
    pub all_day: bool,
    pub location: Option<String>,
    pub href: String,
    pub etag: Option<String>,
}

/// Parse calendar list from PROPFIND response
fn parse_calendar_list(xml: &str, base_url: &str) -> Result<Vec<CalendarInfo>> {
    let mut calendars = Vec::new();

    // Simple XML parsing - look for response elements
    // Handle both <d:response> and <D:response> prefixes
    let response_splits: Vec<&str> = if xml.contains("<d:response>") {
        xml.split("<d:response>").skip(1).collect()
    } else {
        xml.split("<D:response>").skip(1).collect()
    };

    for response_block in response_splits {
        let href = extract_tag_content(response_block, "d:href")
            .or_else(|| extract_tag_content(response_block, "D:href"));

        let displayname = extract_tag_content(response_block, "d:displayname")
            .or_else(|| extract_tag_content(response_block, "D:displayname"));

        // Check if it's a calendar (has calendar in resourcetype)
        let block_lower = response_block.to_lowercase();
        let is_calendar = block_lower.contains("calendar") && block_lower.contains("resourcetype");

        // Check if it supports VEVENT (case insensitive)
        let supports_events = block_lower.contains("vevent");

        log::debug!("CalDAV parse: href={:?} name={:?} is_calendar={} supports_events={}",
                   href, displayname, is_calendar, supports_events);

        if let (Some(href), Some(name)) = (href, displayname) {
            // Skip the base calendar collection itself
            let path = href.trim_start_matches(base_url.trim_end_matches('/'));
            let path = path.trim_start_matches("/remote.php/dav/calendars/");

            // Extract just the calendar name part
            let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            log::debug!("CalDAV parse: path parts={:?}", parts);

            if parts.len() >= 2 && is_calendar {
                calendars.push(CalendarInfo {
                    name: name.to_string(),
                    path: parts[1].to_string(),
                    supports_events,
                });
            } else if parts.len() == 1 && is_calendar && !parts[0].is_empty() {
                // Some setups might have simpler path structure
                calendars.push(CalendarInfo {
                    name: name.to_string(),
                    path: parts[0].to_string(),
                    supports_events,
                });
            }
        }
    }

    Ok(calendars)
}

/// Parse events from REPORT response
fn parse_events_response(xml: &str) -> Result<Vec<CalendarEvent>> {
    let mut events = Vec::new();

    // Split by response elements
    for response_block in xml.split("<d:response>").skip(1) {
        let href = extract_tag_content(response_block, "d:href")
            .or_else(|| extract_tag_content(response_block, "D:href"))
            .unwrap_or_default();

        let etag = extract_tag_content(response_block, "d:getetag")
            .or_else(|| extract_tag_content(response_block, "D:getetag"));

        // Extract calendar-data (iCalendar content)
        let ical_data = extract_tag_content(response_block, "cal:calendar-data")
            .or_else(|| extract_tag_content(response_block, "c:calendar-data"))
            .or_else(|| extract_tag_content(response_block, "C:calendar-data"));

        if let Some(ical) = ical_data {
            if let Some(event) = parse_icalendar_event(&ical, &href, etag) {
                events.push(event);
            }
        }
    }

    Ok(events)
}

/// Parse a single iCalendar VEVENT
fn parse_icalendar_event(ical: &str, href: &str, etag: Option<String>) -> Option<CalendarEvent> {
    // Find VEVENT block
    let vevent_start = ical.find("BEGIN:VEVENT")?;
    let vevent_end = ical.find("END:VEVENT")?;
    let vevent = &ical[vevent_start..vevent_end];

    let uid = extract_ical_property(vevent, "UID")?;
    let summary = extract_ical_property(vevent, "SUMMARY").unwrap_or_else(|| "(No title)".to_string());
    let description = extract_ical_property(vevent, "DESCRIPTION");
    let location = extract_ical_property(vevent, "LOCATION");

    // Parse start time
    let (start, all_day) = parse_ical_datetime(vevent, "DTSTART")?;

    // Parse end time (optional)
    let end = parse_ical_datetime(vevent, "DTEND").map(|(dt, _)| dt);

    Some(CalendarEvent {
        uid,
        summary,
        description,
        start,
        end,
        all_day,
        location,
        href: href.to_string(),
        etag,
    })
}

/// Extract content between XML tags
fn extract_tag_content(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);

    let start = xml.find(&open_tag)?;
    let content_start = start + open_tag.len();
    let end = xml[content_start..].find(&close_tag)?;

    let content = &xml[content_start..content_start + end];
    // Decode XML entities
    let decoded = content
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");

    Some(decoded.trim().to_string())
}

/// Extract an iCalendar property value
fn extract_ical_property(vevent: &str, property: &str) -> Option<String> {
    for line in vevent.lines() {
        let line = line.trim();
        // Handle properties with parameters like DTSTART;VALUE=DATE:20260101
        if line.starts_with(property) {
            if let Some(colon_pos) = line.find(':') {
                let value = &line[colon_pos + 1..];
                // Handle line continuations (lines starting with space/tab)
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// Parse an iCalendar datetime property
fn parse_ical_datetime(vevent: &str, property: &str) -> Option<(NaiveDateTime, bool)> {
    for line in vevent.lines() {
        let line = line.trim();
        if line.starts_with(property) {
            let is_date_only = line.contains("VALUE=DATE") && !line.contains("VALUE=DATE-TIME");

            if let Some(colon_pos) = line.find(':') {
                let value = line[colon_pos + 1..].trim();

                if is_date_only {
                    // Date only: YYYYMMDD
                    if value.len() >= 8 {
                        let year = value[0..4].parse().ok()?;
                        let month = value[4..6].parse().ok()?;
                        let day = value[6..8].parse().ok()?;
                        let date = NaiveDate::from_ymd_opt(year, month, day)?;
                        return Some((date.and_hms_opt(0, 0, 0)?, true));
                    }
                } else {
                    // DateTime: YYYYMMDDTHHMMSS or YYYYMMDDTHHMMSSZ
                    let value = value.trim_end_matches('Z');
                    if value.len() >= 15 {
                        let year = value[0..4].parse().ok()?;
                        let month = value[4..6].parse().ok()?;
                        let day = value[6..8].parse().ok()?;
                        let hour = value[9..11].parse().ok()?;
                        let minute = value[11..13].parse().ok()?;
                        let second = value[13..15].parse().ok()?;
                        let date = NaiveDate::from_ymd_opt(year, month, day)?;
                        let time = NaiveTime::from_hms_opt(hour, minute, second)?;
                        return Some((NaiveDateTime::new(date, time), false));
                    }
                }
            }
        }
    }
    None
}

/// Initiate Nextcloud Login Flow v2
pub async fn initiate_login_flow(nextcloud_url: &str) -> Result<LoginFlowInit> {
    let client = Client::new();
    let endpoint = format!("{}/index.php/login/v2", nextcloud_url.trim_end_matches('/'));

    let response = client
        .post(&endpoint)
        .send()
        .await
        .context("Failed to initiate login flow")?;

    if !response.status().is_success() {
        return Err(anyhow!("Login flow returned status {}", response.status()));
    }

    response
        .json()
        .await
        .context("Failed to parse login flow response")
}

/// Poll for login completion
pub async fn poll_login_completion(poll_endpoint: &str, poll_token: &str) -> Result<Option<LoginFlowResult>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let response = client
        .post(poll_endpoint)
        .form(&[("token", poll_token)])
        .send()
        .await;

    match response {
        Ok(r) if r.status().is_success() => {
            let result: LoginFlowResult = r.json().await.context("Failed to parse poll response")?;
            Ok(Some(result))
        }
        Ok(r) if r.status().as_u16() == 404 => {
            // Not yet completed
            Ok(None)
        }
        Ok(r) => Err(anyhow!("Poll returned status {}", r.status())),
        Err(e) if e.is_timeout() => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Parse file list from WebDAV PROPFIND response
fn parse_file_list(xml: &str, base_path: &str) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    let base_path = base_path.trim_matches('/');

    // Split by response elements (handle both d: and D: prefixes)
    let response_splits: Vec<&str> = if xml.contains("<d:response>") {
        xml.split("<d:response>").skip(1).collect()
    } else {
        xml.split("<D:response>").skip(1).collect()
    };

    for response_block in response_splits {
        let href = extract_tag_content(response_block, "d:href")
            .or_else(|| extract_tag_content(response_block, "D:href"));

        let displayname = extract_tag_content(response_block, "d:displayname")
            .or_else(|| extract_tag_content(response_block, "D:displayname"));

        let content_type = extract_tag_content(response_block, "d:getcontenttype")
            .or_else(|| extract_tag_content(response_block, "D:getcontenttype"));

        let size_str = extract_tag_content(response_block, "oc:size")
            .or_else(|| extract_tag_content(response_block, "d:getcontentlength"))
            .or_else(|| extract_tag_content(response_block, "D:getcontentlength"));
        let size = size_str.and_then(|s| s.parse().ok()).unwrap_or(0);

        // Check if it's a directory (has collection in resourcetype)
        let block_lower = response_block.to_lowercase();
        let is_directory = block_lower.contains("<d:collection") || block_lower.contains("<collection");

        if let Some(href) = href {
            // Extract path from href
            // href is like /remote.php/dav/files/username/path/to/file
            let path = if let Some(pos) = href.find("/remote.php/dav/files/") {
                let after_files = &href[pos + "/remote.php/dav/files/".len()..];
                // Skip username
                if let Some(slash_pos) = after_files.find('/') {
                    after_files[slash_pos..].trim_matches('/').to_string()
                } else {
                    String::new()
                }
            } else {
                href.trim_matches('/').to_string()
            };

            // Skip the directory itself (when path equals base_path)
            if path == base_path || path.is_empty() {
                continue;
            }

            let name = displayname.unwrap_or_else(|| {
                path.rsplit('/').next().unwrap_or(&path).to_string()
            });

            files.push(FileInfo {
                name,
                path,
                is_directory,
                size,
                content_type,
            });
        }
    }

    // Sort: directories first, then by name
    files.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(files)
}
