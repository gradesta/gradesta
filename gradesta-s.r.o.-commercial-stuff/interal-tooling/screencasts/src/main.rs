mod screencasts;
use anyhow;
use screencasts::publish::publish;
use screencasts::inspect::inspect;
use screencasts::print_label::print_label;

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
                .about("Publishes screencasts to s3.")
                .arg(arg!(<BLOG_POST>))
                .arg(arg!(<VIDEO_FILE>... "Use non-existant paths (for example 'skip') in place of mkv files to skip over previously uploaded screencast tags")),
        )
        .subcommand(
            Command::new("inspect-blogpost")
                .about("Inspects a blogpost and shows information about the screencasts listed in it.")
                .arg(arg!(<BLOG_POST>))
        )
        .subcommand(
            Command::new("label")
                .about("Creates a new screencast label and prints it to stdout")
        )
        .get_matches();

    match matches.subcommand() {
        Some(("publish", sub_matches)) => {
            let blog_post_path: &String = sub_matches.get_one::<String>("BLOG_POST").unwrap();
            let video_files = sub_matches.get_many::<String>("VIDEO_FILE").unwrap();
            let video_files: Vec<&str> = video_files.map(|s| s.as_ref()).collect();
            publish(blog_post_path, video_files).await?;
        }
        Some(("inspect-blogpost", sub_matches)) => {
            let blog_post_path: &String = sub_matches.get_one::<String>("BLOG_POST").unwrap();
            inspect(blog_post_path).await?;
        }
        Some(("label", _)) => {
            print_label().await?;
        },
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    };
    return Ok(());
}
