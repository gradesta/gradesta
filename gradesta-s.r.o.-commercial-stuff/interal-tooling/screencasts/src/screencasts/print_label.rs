use anyhow;
use chrono::prelude::*;
use uuid::Uuid;

pub async fn print_label() -> anyhow::Result<()> {
    let now = Local::now();
    println!(
        "{}-{:02}-{:02}-{}",
        now.year(),
        now.month(),
        now.day(),
        Uuid::new_v4(),
    );
    Ok(())
}
