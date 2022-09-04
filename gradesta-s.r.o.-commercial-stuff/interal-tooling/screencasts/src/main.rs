mod screencasts;
use anyhow;
use screencasts::fetch_duration::fetch_and_save_duration;
use screencasts::inspect::inspect;
use screencasts::print_label::print_label;
use screencasts::publish::publish;

extern crate clap;
use clap::{arg, command, Arg, ArgAction, Command};

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
            Command::new("fetch-durations")
                .about("Fetches screencast durations from their web source for screencasts who's urls have been put in their metadata.")
                .arg(arg!(<METADATA>... "List of metadata files to fetch duration for"))
                .arg(Arg::new("missing")
                     .long("missing")
                     .help("Only fetch duration if missing")
                     .action(ArgAction::SetTrue),
            )
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
        Some(("fetch-durations", sub_matches)) => {
            let metadata_files = sub_matches.get_many::<String>("METADATA").unwrap();
            let missing = *sub_matches.get_one::<bool>("missing").unwrap();
            for metadata in metadata_files {
                fetch_and_save_duration(metadata, missing).await?;
            }
        }
        Some(("inspect-blogpost", sub_matches)) => {
            let blog_post_path: &String = sub_matches.get_one::<String>("BLOG_POST").unwrap();
            inspect(blog_post_path).await?;
        }
        Some(("label", _)) => {
            print_label().await?;
        }
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    };
    return Ok(());
}
