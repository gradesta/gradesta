use super::config::load_config;
use super::parsing::load_screencasts_from_blogpost;
use super::save_screencast_metadata::save_screencast_metadata;
use super::transcode_and_upload::transcode_and_upload;

use anyhow::{anyhow, Context};
use daggy::{Dag, NodeIndex};
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::process::ExitStatus;
use tokio::process::{Child, Command};

pub async fn publish(blog_post_path: &str, video_files: Vec<&str>) -> anyhow::Result<()> {
    let blogpost = fs::read_to_string(blog_post_path).unwrap();
    let config = load_config(blog_post_path).await?;

    let mut screencasts = load_screencasts_from_blogpost(&blogpost)?;
    let transcode = transcode_and_upload(&mut screencasts, video_files, &config);
    save_screencast_metadata(&mut screencasts, &config).await?;
    match transcode {
        Ok(commands) => {
            run_command_dag(commands).await?;
            Ok(())
        }
        Err(e) => Err(anyhow!("{}", e)),
    }
}

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

async fn run_command_dag(
    commands: Dag<RefCell<Command>, ()>,
) -> anyhow::Result<HashMap<NodeIndex, ExitStatus>> {
    let mut command_results = HashMap::new();
    // 1. Collect and run zero dependency targets
    let mut running_commands = FuturesUnordered::new();
    use daggy::Walker;
    let mut launched_commands = HashSet::new();
    for node in commands.graph().node_indices() {
        if commands.parents(node).walk_next(&commands) == None {
            launched_commands.insert(node);
            let command = commands.node_weight(node).unwrap();
            let running_command = run_command_and_return_with_node(command, node);
            running_commands.push(running_command);
        }
    }
    // 2. As targets are completed, run their children if dependencies are fulfilled
    while let Some(res) = running_commands.next().await {
        match res {
            Err(e) => return Err(e),
            Ok((exit_status, node)) => {
                println!("Node {:?} complete with exit status {}", node, exit_status);
                command_results.insert(node, exit_status);
                for (_, child) in commands.children(node).iter(&commands) {
                    let mut dependencies_fulfilled = true;
                    for (_, parent) in commands.parents(child).iter(&commands) {
                        if !command_results.contains_key(&parent) {
                            dependencies_fulfilled = false;
                            break;
                        }
                    }
                    if dependencies_fulfilled {
                        if !launched_commands.contains(&child) {
                            launched_commands.insert(child);
                            let command = commands.node_weight(child).unwrap();
                            let running_command = run_command_and_return_with_node(command, child);
                            running_commands.push(running_command);
                        }
                    }
                }
            }
        };
    }
    return Ok(command_results);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_dag_with_non_existant_command() {
        let mut cdag = Dag::new();
        let not_in_path = RefCell::new(Command::new("not-in-path"));
        cdag.add_node(not_in_path);
        assert_eq!(format!("{:?}", cdag), "Dag { graph: Graph { Ty: \"Directed\", node_count: 1, edge_count: 0, node weights: {0: RefCell { value: Command { std: \"not-in-path\", kill_on_drop: false } }} }, cycle_state: DfsSpace { dfs: Dfs { stack: [], discovered: FixedBitSet { data: [], length: 0 } } } }");
        match run_command_dag(cdag).await{
            Err(e) => assert_eq!(format!("{:?}", e), "Error running command Command { std: \"not-in-path\", kill_on_drop: false }\n\nCaused by:\n    No such file or directory (os error 2)"),
            Ok(_) => assert!(false)
        };
    }

    #[tokio::test]
    async fn test_run_command_dag() {
        let mut cdag = Dag::new();
        let echo = RefCell::new(Command::new("echo"));
        {
            let mut echo = echo.borrow_mut();
            echo.arg("Hi");
        }
        let c1 = cdag.add_node(echo);
        let fail_cat = RefCell::new(Command::new("cat"));
        {
            let mut fail_cat = fail_cat.borrow_mut();
            fail_cat.arg("non-existant-file");
        }
        let (_, c2) = cdag.add_child(c1, (), fail_cat);
        assert_eq!(format!("{:?}", cdag), "Dag { graph: Graph { Ty: \"Directed\", node_count: 2, edge_count: 1, edges: (0, 1), node weights: {0: RefCell { value: Command { std: \"echo\" \"Hi\", kill_on_drop: false } }, 1: RefCell { value: Command { std: \"cat\" \"non-existant-file\", kill_on_drop: false } }} }, cycle_state: DfsSpace { dfs: Dfs { stack: [], discovered: FixedBitSet { data: [], length: 0 } } } }");
        let results = run_command_dag(cdag).await.unwrap();
        assert_eq!(results.get(&c1).unwrap().success(), true);
        assert_eq!(results.get(&c2).unwrap().success(), false);
    }
}
