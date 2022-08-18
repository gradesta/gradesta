use anyhow;
use uuid::Uuid;
use chrono::prelude::*;

pub async fn print_label() -> anyhow::Result<()> {
    let now = Local::now();
    println!(
        "{}-{}-{}-{}",
        now.year(),
        now.month(),
        now.day(),
        Uuid::new_v4(),
    );
    Ok(())
}
