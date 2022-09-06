use anyhow;
use time::OffsetDateTime;
use uuid::Uuid;

pub async fn print_label() -> anyhow::Result<()> {
    let now = OffsetDateTime::now_utc();
    println!(
        "{}-{:02}-{:02}-{}",
        now.year(),
        <u8>::from(now.month()),
        now.day(),
        Uuid::new_v4(),
    );
    Ok(())
}
