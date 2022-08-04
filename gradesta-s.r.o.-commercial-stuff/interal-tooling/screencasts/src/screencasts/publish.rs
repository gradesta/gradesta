use super::transcode_and_upload::transcode_and_upload;
use anyhow::anyhow;
use std::fs;

pub async fn publish(blog_post_path: &str, video_files: Vec<&str>) -> anyhow::Result<()> {
    let blog_post = fs::read_to_string(blog_post_path).unwrap();
    let transcoded = transcode_and_upload(&blog_post, video_files);
    match transcoded {
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
                    }
                    None => (),
                };
                // 2a. When a target completes, add its children to to the pool of targets to run.
            }
            return Ok(());
        }
        Err(e) => {
            eprintln!("{}", e);
            Err(anyhow!(""))
        }
    }
}
