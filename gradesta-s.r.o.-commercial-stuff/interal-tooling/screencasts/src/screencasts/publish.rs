use super::transcode_and_upload::transcode_and_upload;
use anyhow::anyhow;
use anyhow::Context;
use daggy::{Dag, NodeIndex};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::cell::RefCell;
use std::fs;
use std::process::ExitStatus;
use tokio::process::{Child, Command};

async fn run_command_and_return_with_node(
    command: &RefCell<Command>,
    node: NodeIndex,
) -> anyhow::Result<(ExitStatus, NodeIndex)> {
    let mut command = command.borrow_mut();
    let mut child: Child = command
        .spawn()
        .context(format!("Error running command {:?}", command))?;
    let command_output = child.wait().await.context("")?;
    Ok((command_output, node))
}

pub async fn publish(blog_post_path: &str, video_files: Vec<&str>) -> anyhow::Result<()> {
    let blog_post = fs::read_to_string(blog_post_path).unwrap();
    let transcoded = transcode_and_upload(&blog_post, video_files);
    match transcoded {
        Ok(commands) => run_command_dag(commands).await,
        Err(e) => {
            eprintln!("{}", e);
            Err(anyhow!(""))
        }
    }
}

async fn run_command_dag(commands: Dag<RefCell<Command>, ()>) -> anyhow::Result<()> {
    // 1. Collect zero dependency targets
    let mut target_queue = vec![];
    let mut running_commands = FuturesUnordered::new();
    use daggy::Walker;
    use std::collections::HashSet;
    let mut launched_commands = HashSet::new();
    let mut completed_commands = HashSet::new();
    for node in commands.graph().node_indices() {
        if commands.parents(node).walk_next(&commands) == None {
            target_queue.push(node);
        }
    }
    // 2. Loop thorough collected targets, running them
    while target_queue.len() > 0 {
        match target_queue.pop() {
            Some(node) => {
                if !launched_commands.contains(&node) {
                    launched_commands.insert(node);
                    let command = commands.node_weight(node).unwrap();
                    let running_command = run_command_and_return_with_node(command, node);
                    running_commands.push(running_command);
                }
            }
            None => (),
        };
    }
    // 2a. When a target completes, add its children to to the pool of targets to run.
    while let Some(res) = running_commands.next().await {
        match res {
            Err(e) => return Err(e),
            Ok((exit_status, node)) => {
                println!("Node {:?} complete with exit status {}", node, exit_status);
                completed_commands.insert(node);
                for (_, child) in commands.children(node).iter(&commands) {
                    let mut dependencies_fulfilled = true;
                    for (_, parent) in commands.parents(child).iter(&commands) {
                        if !completed_commands.contains(&parent) {
                            dependencies_fulfilled = false;
                            break;
                        }
                    }
                    if dependencies_fulfilled {
                        if !launched_commands.contains(&node) {
                            launched_commands.insert(node);
                            let command = commands.node_weight(node).unwrap();
                            let running_command = run_command_and_return_with_node(command, node);
                            running_commands.push(running_command);
                        }
                    }
                }
            }
        };
    }
    return Ok(());
}
