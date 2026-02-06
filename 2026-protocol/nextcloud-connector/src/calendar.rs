//! Calendar module - CalDAV integration with year/month/day hierarchy

use anyhow::Result;
use chrono::{Datelike, Local, NaiveDate};
use futures_util::SinkExt;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

use crate::nextcloud::{CalendarEvent, NextcloudClient};
use crate::protocol::*;

/// Generate deterministic hash for calendar vertices
fn calendar_hash(identity: &str, name: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("calendar:{}:{}", identity, name).hash(&mut hasher);
    hasher.finish()
}

fn year_hash(identity: &str, year: i32) -> u64 {
    calendar_hash(identity, &format!("year:{}", year))
}

fn month_hash(identity: &str, year: i32, month: u32) -> u64 {
    calendar_hash(identity, &format!("month:{}:{}", year, month))
}

fn day_hash(identity: &str, year: i32, month: u32, day: u32) -> u64 {
    calendar_hash(identity, &format!("day:{}-{:02}-{:02}", year, month, day))
}

fn event_hash(event_uid: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    format!("event:{}", event_uid).hash(&mut hasher);
    hasher.finish()
}

/// Parse calendar path to determine what to show
#[derive(Debug)]
enum CalendarPath {
    Root,
    Year(i32),
    Month(i32, u32),
    Day(i32, u32, u32),
}

fn parse_calendar_path(path: &str) -> CalendarPath {
    let parts: Vec<&str> = path.trim_matches('/').split('/').filter(|s| !s.is_empty()).collect();

    match parts.as_slice() {
        [] => CalendarPath::Root,
        [year] => {
            if let Ok(y) = year.parse::<i32>() {
                CalendarPath::Year(y)
            } else {
                CalendarPath::Root
            }
        }
        [year, month] => {
            if let (Ok(y), Ok(m)) = (year.parse::<i32>(), month.parse::<u32>()) {
                if m >= 1 && m <= 12 {
                    CalendarPath::Month(y, m)
                } else {
                    CalendarPath::Root
                }
            } else {
                CalendarPath::Root
            }
        }
        [year, month, day] => {
            if let (Ok(y), Ok(m), Ok(d)) = (year.parse::<i32>(), month.parse::<u32>(), day.parse::<u32>()) {
                if m >= 1 && m <= 12 && d >= 1 && d <= 31 {
                    CalendarPath::Day(y, m, d)
                } else {
                    CalendarPath::Root
                }
            } else {
                CalendarPath::Root
            }
        }
        _ => CalendarPath::Root,
    }
}

/// Handle calendar landmark requests
pub async fn handle_landmark<W, S>(
    state: &Arc<Mutex<S>>,
    write: &mut W,
    action_id: u64,
    path: &str,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
    S: HasIdentity,
{
    let (identity, nc) = {
        let s = state.lock().await;
        (s.get_identity(), s.get_nextcloud())
    };

    log::info!("Calendar landmark: path='{}', identity='{}'", path, identity);

    let parsed = parse_calendar_path(path);
    log::info!("Parsed calendar path: {:?}", parsed);

    match parsed {
        CalendarPath::Root => send_years_listing(&identity, write, action_id, nc.as_ref()).await,
        CalendarPath::Year(year) => send_months_listing(&identity, write, action_id, year, nc.as_ref()).await,
        CalendarPath::Month(year, month) => send_days_listing(&identity, write, action_id, year, month, nc.as_ref()).await,
        CalendarPath::Day(year, month, day) => send_day_with_events(&identity, write, action_id, year, month, day, nc.as_ref()).await,
    }
}

/// Trait for accessing identity and nextcloud client from state
pub trait HasIdentity {
    fn get_identity(&self) -> String;
    fn get_nextcloud(&self) -> Option<NextcloudClient>;
}

/// Send calendar root with year listing
/// Includes years, months, and days (without events - events loaded when navigating to specific month)
async fn send_years_listing<W>(
    identity: &str,
    write: &mut W,
    action_id: u64,
    nc: Option<&NextcloudClient>,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Send context
    let landmark = format!("nextcloud://{}/calendar/", identity);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Get current year and show ±2 years
    let current_year = Local::now().year();
    let years: Vec<i32> = ((current_year - 2)..=(current_year + 2)).collect();

    let month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December"
    ];

    // Calendar root vertex
    let root_id = calendar_hash(identity, "root");
    let root_msg = encode_set_vertex_label(action_id, root_id, "text/plain", b"Calendar");
    write.send(Message::Binary(root_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Root edges: south to current year
    let current_year_id = year_hash(identity, current_year);
    let root_edges = encode_set_edges(action_id, root_id, 0, 0, 0, current_year_id, 0, 0, 0);
    write.send(Message::Binary(root_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Send year, month, and day vertices
    for (i, &year) in years.iter().enumerate() {
        let year_id = year_hash(identity, year);
        let label = format!("{}", year);
        let year_msg = encode_set_vertex_label(action_id, year_id, "text/plain", label.as_bytes());
        write.send(Message::Binary(year_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Year edges: west/east to neighboring years, north to root, south to January
        let west = if i > 0 { year_hash(identity, years[i - 1]) } else { 0 };
        let east = if i < years.len() - 1 { year_hash(identity, years[i + 1]) } else { 0 };
        let north = if year == current_year { root_id } else { 0 };
        let south = month_hash(identity, year, 1); // January

        let year_edges = encode_set_edges(action_id, year_id, west, east, north, south, 0, 0, 0);
        write.send(Message::Binary(year_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Send month vertices with days
        for month in 1u32..=12 {
            let month_id = month_hash(identity, year, month);
            let month_label = month_names[(month - 1) as usize];
            let month_msg = encode_set_vertex_label(action_id, month_id, "text/plain", month_label.as_bytes());
            write.send(Message::Binary(month_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

            // Month edges: west/east to neighboring months, north to year, south to day 1
            let m_west = if month > 1 { month_hash(identity, year, month - 1) } else { 0 };
            let m_east = if month < 12 { month_hash(identity, year, month + 1) } else { 0 };
            let m_north = if month == 1 { year_id } else { 0 };
            let m_south = day_hash(identity, year, month, 1);

            let month_edges = encode_set_edges(action_id, month_id, m_west, m_east, m_north, m_south, 0, 0, 0);
            write.send(Message::Binary(month_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

            // Send day vertices for this month
            let num_days = days_in_month(year, month);
            for day in 1u32..=num_days {
                let day_id = day_hash(identity, year, month, day);
                let date = NaiveDate::from_ymd_opt(year, month, day);
                let weekday = date.map(|d| d.weekday().to_string()).unwrap_or_default();
                let day_label = format!("{} {}", weekday, day);

                let day_msg = encode_set_vertex_label(action_id, day_id, "text/plain", day_label.as_bytes());
                write.send(Message::Binary(day_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

                // Day edges: west/east to neighboring days, north to month
                let d_west = if day > 1 { day_hash(identity, year, month, day - 1) } else { 0 };
                let d_east = if day < num_days { day_hash(identity, year, month, day + 1) } else { 0 };
                let d_north = if day == 1 { month_id } else { 0 };

                // No events in initial load (they'll be fetched when navigating to specific day/month)
                let day_edges = encode_set_edges(action_id, day_id, d_west, d_east, d_north, 0, 0, 0, 0);
                write.send(Message::Binary(day_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
            }
        }
    }

    // If we have a nextcloud client, fetch events for current month and add them
    if let Some(nc) = nc {
        let current_month = Local::now().month();
        log::info!("Fetching events for current month {}-{:02}", current_year, current_month);
        if let Ok(events) = fetch_month_events(nc, current_year, current_month).await {
            send_events_for_month(identity, write, action_id, current_year, current_month, &events).await?;
        }
    }

    log::info!("Sent calendar years listing with months and days (action={})", action_id);
    Ok(())
}

/// Send months for a year (includes days)
async fn send_months_listing<W>(
    identity: &str,
    write: &mut W,
    action_id: u64,
    year: i32,
    nc: Option<&NextcloudClient>,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Send context
    let landmark = format!("nextcloud://{}/calendar/{}/", identity, year);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    let month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December"
    ];

    // Send year vertex as reference point
    let year_id = year_hash(identity, year);
    let year_msg = encode_set_vertex_label(action_id, year_id, "text/plain", format!("{}", year).as_bytes());
    write.send(Message::Binary(year_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Year edges
    let prev_year_id = year_hash(identity, year - 1);
    let next_year_id = year_hash(identity, year + 1);
    let jan_id = month_hash(identity, year, 1);
    let year_edges = encode_set_edges(action_id, year_id, prev_year_id, next_year_id, 0, jan_id, 0, 0, 0);
    write.send(Message::Binary(year_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Send month vertices with days
    for month in 1u32..=12 {
        let month_id = month_hash(identity, year, month);
        let label = month_names[(month - 1) as usize];
        let month_msg = encode_set_vertex_label(action_id, month_id, "text/plain", label.as_bytes());
        write.send(Message::Binary(month_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Month edges: west/east to neighboring months, north to year, south to day 1
        let west = if month > 1 { month_hash(identity, year, month - 1) } else { 0 };
        let east = if month < 12 { month_hash(identity, year, month + 1) } else { 0 };
        let north = if month == 1 { year_id } else { 0 };
        let south = day_hash(identity, year, month, 1);

        let month_edges = encode_set_edges(action_id, month_id, west, east, north, south, 0, 0, 0);
        write.send(Message::Binary(month_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Send day vertices for this month
        let num_days = days_in_month(year, month);
        for day in 1u32..=num_days {
            let day_id = day_hash(identity, year, month, day);
            let date = NaiveDate::from_ymd_opt(year, month, day);
            let weekday = date.map(|d| d.weekday().to_string()).unwrap_or_default();
            let day_label = format!("{} {}", weekday, day);

            let day_msg = encode_set_vertex_label(action_id, day_id, "text/plain", day_label.as_bytes());
            write.send(Message::Binary(day_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

            // Day edges: west/east to neighboring days, north to month
            let d_west = if day > 1 { day_hash(identity, year, month, day - 1) } else { 0 };
            let d_east = if day < num_days { day_hash(identity, year, month, day + 1) } else { 0 };
            let d_north = if day == 1 { month_id } else { 0 };

            let day_edges = encode_set_edges(action_id, day_id, d_west, d_east, d_north, 0, 0, 0, 0);
            write.send(Message::Binary(day_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
    }

    // Fetch and send events for current month if we have nextcloud client
    if let Some(nc) = nc {
        let current_month = Local::now().month();
        if let Ok(events) = fetch_month_events(nc, year, current_month).await {
            send_events_for_month(identity, write, action_id, year, current_month, &events).await?;
        }
    }

    log::info!("Sent calendar months listing for {} with days (action={})", year, action_id);
    Ok(())
}

/// Send days for a month
async fn send_days_listing<W>(
    identity: &str,
    write: &mut W,
    action_id: u64,
    year: i32,
    month: u32,
    nc: Option<&NextcloudClient>,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Send context
    let landmark = format!("nextcloud://{}/calendar/{}/{}/", identity, year, month);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Calculate number of days in month
    let num_days = days_in_month(year, month);

    let month_names = [
        "January", "February", "March", "April", "May", "June",
        "July", "August", "September", "October", "November", "December"
    ];

    // Fetch events for the entire month if we have a nextcloud client
    let month_events: Vec<CalendarEvent> = if let Some(nc) = nc {
        let start_date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let end_date = NaiveDate::from_ymd_opt(year, month, num_days).unwrap();
        match nc.fetch_events("personal", start_date, end_date).await {
            Ok(events) => {
                log::info!("Fetched {} events for {}-{:02}", events.len(), year, month);
                events
            }
            Err(e) => {
                log::warn!("Failed to fetch events: {}", e);
                // Try fetching from all calendars
                match nc.fetch_all_events_for_day(start_date).await {
                    Ok(events) => events,
                    Err(_) => Vec::new(),
                }
            }
        }
    } else {
        Vec::new()
    };

    // Group events by day
    let mut events_by_day: std::collections::HashMap<u32, Vec<&CalendarEvent>> = std::collections::HashMap::new();
    for event in &month_events {
        let event_day = event.start.date().day();
        events_by_day.entry(event_day).or_default().push(event);
    }

    // Send month vertex as reference
    let month_id = month_hash(identity, year, month);
    let month_label = month_names[(month - 1) as usize];
    let month_msg = encode_set_vertex_label(action_id, month_id, "text/plain", month_label.as_bytes());
    write.send(Message::Binary(month_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Month edges
    let west = if month > 1 { month_hash(identity, year, month - 1) } else { 0 };
    let east = if month < 12 { month_hash(identity, year, month + 1) } else { 0 };
    let north = year_hash(identity, year);
    let south = day_hash(identity, year, month, 1);
    let month_edges = encode_set_edges(action_id, month_id, west, east, north, south, 0, 0, 0);
    write.send(Message::Binary(month_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Send day vertices
    for day in 1u32..=num_days {
        let day_id = day_hash(identity, year, month, day);
        let date = NaiveDate::from_ymd_opt(year, month, day);
        let weekday = date.map(|d| d.weekday().to_string()).unwrap_or_default();

        // Add event count to day label if there are events
        let day_events = events_by_day.get(&day);
        let event_count = day_events.map(|e| e.len()).unwrap_or(0);
        let label = if event_count > 0 {
            format!("{} {} ({})", weekday, day, event_count)
        } else {
            format!("{} {}", weekday, day)
        };

        let day_msg = encode_set_vertex_label(action_id, day_id, "text/plain", label.as_bytes());
        write.send(Message::Binary(day_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Day edges: west/east to neighboring days, north to month
        let west = if day > 1 { day_hash(identity, year, month, day - 1) } else { 0 };
        let east = if day < num_days { day_hash(identity, year, month, day + 1) } else { 0 };
        let north = if day == 1 { month_id } else { 0 };

        // Connect first event via down edge
        let down = if let Some(events) = day_events {
            if let Some(first_event) = events.first() {
                event_hash(&first_event.uid)
            } else {
                0
            }
        } else {
            0
        };

        let day_edges = encode_set_edges(action_id, day_id, west, east, north, 0, 0, down, 0);
        write.send(Message::Binary(day_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Send event vertices for this day
        if let Some(events) = day_events {
            for (i, event) in events.iter().enumerate() {
                let event_id = event_hash(&event.uid);

                // Format event label with time and title
                let time_str = if event.all_day {
                    "All day".to_string()
                } else {
                    event.start.format("%H:%M").to_string()
                };
                let event_label = format!("{} {}", time_str, event.summary);

                let event_msg = encode_set_vertex_label(action_id, event_id, "text/plain", event_label.as_bytes());
                write.send(Message::Binary(event_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

                // Event edges: up to day (or previous event), down to next event
                let up = if i == 0 {
                    day_id
                } else {
                    event_hash(&events[i - 1].uid)
                };
                let down = if i + 1 < events.len() {
                    event_hash(&events[i + 1].uid)
                } else {
                    0
                };

                let event_edges = encode_set_edges(action_id, event_id, 0, 0, 0, 0, up, down, 0);
                write.send(Message::Binary(event_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
            }
        }
    }

    log::info!("Sent calendar days listing for {}-{:02} with {} events (action={})", year, month, month_events.len(), action_id);
    Ok(())
}

/// Send a specific day with its events
async fn send_day_with_events<W>(
    identity: &str,
    write: &mut W,
    action_id: u64,
    year: i32,
    month: u32,
    day: u32,
    nc: Option<&NextcloudClient>,
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    // Send context
    let landmark = format!("nextcloud://{}/calendar/{}/{}/{}/", identity, year, month, day);
    let ctx_msg = encode_set_context(action_id, &landmark);
    write.send(Message::Binary(ctx_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Fetch events for this day
    let events: Vec<CalendarEvent> = if let Some(nc) = nc {
        let date = NaiveDate::from_ymd_opt(year, month, day).unwrap();
        match nc.fetch_all_events_for_day(date).await {
            Ok(events) => {
                log::info!("Fetched {} events for {}-{:02}-{:02}", events.len(), year, month, day);
                events
            }
            Err(e) => {
                log::warn!("Failed to fetch events for day: {}", e);
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // Send day vertex
    let day_id = day_hash(identity, year, month, day);
    let date = NaiveDate::from_ymd_opt(year, month, day);
    let weekday = date.map(|d| d.weekday().to_string()).unwrap_or_default();

    let label = if events.is_empty() {
        format!("{} {} {}, {}", weekday, month_name(month), day, year)
    } else {
        format!("{} {} {}, {} ({} events)", weekday, month_name(month), day, year, events.len())
    };

    let day_msg = encode_set_vertex_label(action_id, day_id, "text/plain", label.as_bytes());
    write.send(Message::Binary(day_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Day edges
    let num_days = days_in_month(year, month);
    let west = if day > 1 { day_hash(identity, year, month, day - 1) } else { 0 };
    let east = if day < num_days { day_hash(identity, year, month, day + 1) } else { 0 };
    let north = month_hash(identity, year, month);

    // Connect first event via down edge
    let down = events.first().map(|e| event_hash(&e.uid)).unwrap_or(0);

    let day_edges = encode_set_edges(action_id, day_id, west, east, north, 0, 0, down, 0);
    write.send(Message::Binary(day_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

    // Send event vertices
    for (i, event) in events.iter().enumerate() {
        let event_id = event_hash(&event.uid);

        // Format event with time, title, and optional description/location
        let time_str = if event.all_day {
            "All day".to_string()
        } else {
            let end_str = event.end.map(|e| format!("-{}", e.format("%H:%M"))).unwrap_or_default();
            format!("{}{}", event.start.format("%H:%M"), end_str)
        };

        let mut event_label = format!("{}: {}", time_str, event.summary);
        if let Some(ref location) = event.location {
            if !location.is_empty() {
                event_label.push_str(&format!("\n📍 {}", location));
            }
        }
        if let Some(ref desc) = event.description {
            if !desc.is_empty() {
                // Truncate description if too long
                let short_desc: String = desc.chars().take(100).collect();
                if desc.len() > 100 {
                    event_label.push_str(&format!("\n{}...", short_desc));
                } else {
                    event_label.push_str(&format!("\n{}", short_desc));
                }
            }
        }

        let event_msg = encode_set_vertex_label(action_id, event_id, "text/plain", event_label.as_bytes());
        write.send(Message::Binary(event_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Event edges: up to day (or previous event), down to next event
        let up = if i == 0 {
            day_id
        } else {
            event_hash(&events[i - 1].uid)
        };
        let down = if i + 1 < events.len() {
            event_hash(&events[i + 1].uid)
        } else {
            0
        };

        let event_edges = encode_set_edges(action_id, event_id, 0, 0, 0, 0, up, down, 0);
        write.send(Message::Binary(event_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
    }

    log::info!("Sent calendar day {}-{:02}-{:02} with {} events (action={})", year, month, day, events.len(), action_id);
    Ok(())
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "Unknown",
    }
}

/// Fetch events for a month from Nextcloud CalDAV
async fn fetch_month_events(
    nc: &NextcloudClient,
    year: i32,
    month: u32,
) -> Result<Vec<crate::nextcloud::CalendarEvent>> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)
        .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;
    let end_day = days_in_month(year, month);
    let end = NaiveDate::from_ymd_opt(year, month, end_day)
        .ok_or_else(|| anyhow::anyhow!("Invalid date"))?;

    log::info!("Fetching events for {}-{:02} (days {} to {})", year, month, 1, end_day);

    // Get all calendars and fetch events from each
    let calendars = nc.list_calendars().await?;
    let mut all_events = Vec::new();

    for cal in calendars {
        if cal.supports_events {
            match nc.fetch_events(&cal.path, start, end).await {
                Ok(events) => {
                    log::info!("Got {} events from calendar '{}'", events.len(), cal.name);
                    all_events.extend(events);
                }
                Err(e) => log::warn!("Failed to fetch events from {}: {}", cal.name, e),
            }
        }
    }

    // Sort by start time
    all_events.sort_by(|a, b| a.start.cmp(&b.start));
    Ok(all_events)
}

/// Send events for a month, updating day vertices to show event counts
async fn send_events_for_month<W>(
    identity: &str,
    write: &mut W,
    action_id: u64,
    year: i32,
    month: u32,
    events: &[crate::nextcloud::CalendarEvent],
) -> Result<()>
where
    W: SinkExt<Message> + Unpin,
    W::Error: std::fmt::Debug,
{
    use std::collections::HashMap;

    // Group events by day
    let mut events_by_day: HashMap<u32, Vec<&crate::nextcloud::CalendarEvent>> = HashMap::new();
    for event in events {
        let event_day = event.start.day();
        // Only include if the event is in this month
        if event.start.month() == month {
            events_by_day.entry(event_day).or_default().push(event);
        }
    }

    log::info!("Found events on {} days for {}-{:02}", events_by_day.len(), year, month);

    // Update day vertices that have events
    let month_id = month_hash(identity, year, month);
    let num_days = days_in_month(year, month);

    for (&day, day_events) in &events_by_day {
        let day_id = day_hash(identity, year, month, day);
        let date = NaiveDate::from_ymd_opt(year, month, day);
        let weekday = date.map(|d| d.weekday().to_string()).unwrap_or_default();

        let label = format!("{} {} ({} events)", weekday, day, day_events.len());
        let day_msg = encode_set_vertex_label(action_id, day_id, "text/plain", label.as_bytes());
        write.send(Message::Binary(day_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Day edges: west/east to neighboring days, north to month, down to first event
        let west = if day > 1 { day_hash(identity, year, month, day - 1) } else { 0 };
        let east = if day < num_days { day_hash(identity, year, month, day + 1) } else { 0 };
        let north = if day == 1 { month_id } else { 0 };
        let down = event_hash(&day_events[0].uid);

        let day_edges = encode_set_edges(action_id, day_id, west, east, north, 0, 0, down, 0);
        write.send(Message::Binary(day_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

        // Send event vertices
        for (i, event) in day_events.iter().enumerate() {
            let event_id = event_hash(&event.uid);

            // Format event
            let time_str = if event.all_day {
                "All day".to_string()
            } else {
                let end_str = event.end.map(|e| format!("-{}", e.format("%H:%M"))).unwrap_or_default();
                format!("{}{}", event.start.format("%H:%M"), end_str)
            };

            let mut event_label = format!("{}: {}", time_str, event.summary);
            if let Some(ref location) = event.location {
                if !location.is_empty() {
                    event_label.push_str(&format!("\n📍 {}", location));
                }
            }

            let event_msg = encode_set_vertex_label(action_id, event_id, "text/plain", event_label.as_bytes());
            write.send(Message::Binary(event_msg)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;

            // Event edges: up to day (or previous event), down to next event
            let up = if i == 0 {
                day_id
            } else {
                event_hash(&day_events[i - 1].uid)
            };
            let down = if i + 1 < day_events.len() {
                event_hash(&day_events[i + 1].uid)
            } else {
                0
            };

            let event_edges = encode_set_edges(action_id, event_id, 0, 0, 0, 0, up, down, 0);
            write.send(Message::Binary(event_edges)).await.map_err(|e| anyhow::anyhow!("{:?}", e))?;
        }
    }

    Ok(())
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
