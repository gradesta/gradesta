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
    pub username: String,
    pub password: String,
    client: Client,
}

impl NextcloudClient {
    pub fn new(url: &str, username: &str, password: &str) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
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

        if !response.status().is_success() && response.status().as_u16() != 207 {
            return Err(anyhow!("CalDAV list calendars failed: {}", response.status()));
        }

        let body = response.text().await?;
        parse_calendar_list(&body, &self.caldav_url())
    }

    /// Fetch events from a calendar within a date range
    pub async fn fetch_events(
        &self,
        calendar_path: &str,
        start: NaiveDate,
        end: NaiveDate,
    ) -> Result<Vec<CalendarEvent>> {
        let url = format!("{}{}", self.caldav_url(), calendar_path.trim_matches('/'));

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

        if !response.status().is_success() && response.status().as_u16() != 207 {
            return Err(anyhow!("CalDAV fetch events failed: {}", response.status()));
        }

        let body = response.text().await?;
        parse_events_response(&body)
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
        let calendars = self.list_calendars().await?;
        let mut all_events = Vec::new();

        for cal in calendars {
            if cal.supports_events {
                match self.fetch_events_for_day(&cal.path, date).await {
                    Ok(events) => all_events.extend(events),
                    Err(e) => log::warn!("Failed to fetch events from {}: {}", cal.name, e),
                }
            }
        }

        // Sort events by start time
        all_events.sort_by(|a, b| a.start.cmp(&b.start));
        Ok(all_events)
    }
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
    for response_block in xml.split("<d:response>").skip(1) {
        let href = extract_tag_content(response_block, "d:href")
            .or_else(|| extract_tag_content(response_block, "D:href"));

        let displayname = extract_tag_content(response_block, "d:displayname")
            .or_else(|| extract_tag_content(response_block, "D:displayname"));

        // Check if it's a calendar (has calendar resourcetype)
        let is_calendar = response_block.contains("calendar") &&
            (response_block.contains("<d:resourcetype>") || response_block.contains("<D:resourcetype>"));

        // Check if it supports VEVENT
        let supports_events = response_block.contains("VEVENT");

        if let (Some(href), Some(name)) = (href, displayname) {
            // Skip the base calendar collection itself
            let path = href.trim_start_matches(base_url.trim_end_matches('/'));
            let path = path.trim_start_matches("/remote.php/dav/calendars/");

            // Extract just the calendar name part
            let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            if parts.len() >= 2 && is_calendar {
                calendars.push(CalendarInfo {
                    name: name.to_string(),
                    path: parts[1].to_string(),
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
