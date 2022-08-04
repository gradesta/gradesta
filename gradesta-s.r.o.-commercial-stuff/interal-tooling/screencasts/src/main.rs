mod screencasts;
use anyhow;
use screencasts::publish::publish;

extern crate clap;
use clap::{arg, command, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = command!() // requires `cargo` feature
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("publish")
                .about("Publishes screencasts to s3")
                .arg(arg!(<BLOG_POST>))
                .arg(arg!(<VIDEO_FILE>...)),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("publish", sub_matches)) => {
            let blog_post_path: &String = sub_matches.get_one::<String>("BLOG_POST").unwrap();
            let video_files = sub_matches.get_many::<String>("VIDEO_FILE").unwrap();
            let video_files: Vec<&str> = video_files.map(|s| s.as_ref()).collect();
            publish(blog_post_path, video_files).await?;
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    };
    return Ok(());
}
