mod screencasts;
use screencasts::transcode_and_upload::{transcode_and_upload};
use std::fs;
use std::process;
use anyhow;

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
                .arg(arg!(<VIDEO_FILE>...))
        )
        .get_matches();

    match matches.subcommand() {
        Some(("publish", sub_matches)) => {
            let blog_post_path: &String = sub_matches.get_one::<String>("BLOG_POST").unwrap();
            let blog_post = fs::read_to_string(blog_post_path).unwrap();

            let video_files = sub_matches.get_many::<String>("VIDEO_FILE").unwrap();
            let video_files: Vec<&str> = video_files.map(|s| s.as_ref()).collect();
            match transcode_and_upload(&blog_post, video_files) {
                Ok(commands) => {
                    // 1. Collect zero dependency targets
                    let mut target_queue = vec![];
                    let mut running_commands = vec![]; // Vec<(Future, NodeId)>
                    use daggy::Walker;
                    use std::collections::HashSet;
                    let mut launched_commands = HashSet::new();
                    for node in commands.graph().node_indices() {
                        if commands.parents(node).walk_next(&commands) == None {
                            target_queue.push(node);
                        }
                    }
                    // 2. Loop thorough collected targets, running them
                    while target_queue.len() > 0 || running_commands.len() > 0 {
                        match target_queue.pop() {
                            Some(node) => {
                                if !launched_commands.contains(&node) {
                                    launched_commands.insert(node);
                                    let mut command = commands.node_weight(node).unwrap().borrow_mut();
                                    let running_command = command.spawn()?;
                                    running_commands.push((running_command, node));

                                }
                            },
                            None => ()
                        };
                    };
                    // 2a. When a target completes, add its children to to the pool of targets to run.
                },
                Err(e) => {
                    eprintln!("{}", e);
                    process::exit(1);
                }
            };

        },
        _ => unreachable!("Exhausted list of subcommands and subcommand_required prevents `None`"),
    };
    return Ok(())
}
