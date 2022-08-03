mod screencasts;
use screencasts::transcode_and_upload::{transcode_and_upload};
use std::fs;
use std::process;

extern crate clap;
use clap::{arg, command, Command};

fn main() {
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
                    use daggy::Walker;
                    for node in commands.graph().node_indices() {
                        if commands.parents(node).walk_next(&commands) == None {
                            target_queue.push(node);
                        }
                    }
                    // 2. Loop thorough collected targets, running them
                    while target_queue.len() > 0 {
                        match target_queue.pop() {
                            Some(node) => {
                                println!("{:?}", commands.node_weight(node));
                            },
                            None => {
                                break;
                            }
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
    }
}
