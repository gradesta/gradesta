use super::config::Config;
use super::parsing::extract_screencast_tags;
use super::screencast::Screencast;
use daggy::{Dag, NodeIndex};
use std::cell::RefCell;
use std::error::Error;
use std::path::Path;
use tokio::process::Command;

/// Takes the contents of a blog post and returns a list of lists of external commands to run.
/// These lists can be run in paralel.
///
pub fn transcode_and_upload<'a>(
    blogpost: &'a str,
    video_files: Vec<&str>,
    config: &Config,
) -> Result<Dag<RefCell<Command>, ()>, Box<dyn Error + 'a>> {
    let (_, mut screencasts) = extract_screencast_tags(blogpost)?;
    if video_files.len() != screencasts.len() {
        return Err(format!("There are {} screencasts refered to in the blog post but {} were passed. Please pass one video file per screencast tag.",
                           screencasts.len(),
                           video_files.len()
        ).into());
    }
    let mut i = 0;
    for video_file in video_files {
        screencasts[i].video_file = Some(video_file.to_string());
        i += 1;
    }
    return build_command_dag(screencasts, config);
}

pub fn build_command_dag(
    screencasts: Vec<Screencast>,
    config: &Config,
) -> Result<Dag<RefCell<Command>, ()>, Box<dyn Error>> {
    let mut commands: Dag<RefCell<Command>, ()> = Dag::new();
    let mut first_transcode: Option<NodeIndex> = None;
    for screencast in screencasts {
        let mkvfile = screencast.video_file.unwrap();
        if !std::path::Path::new(&mkvfile).exists() {
            continue;
        }
        let mp4file_path = Path::new(&mkvfile).with_extension("mp4");
        let path_encoding_error: Box<dyn Error> =
            format!("Invalid character in video file path {}.", &mkvfile).into();
        let mp4file = mp4file_path.to_str().ok_or(path_encoding_error)?;
        let s3path = format!("{}{}.mp4", config.s3_screencasts_path, screencast.id);
        let ffmpeg_cmd = RefCell::new(Command::new("ffmpeg"));
        {
            let mut ffmpeg_cmd_bld = ffmpeg_cmd.borrow_mut();
            ffmpeg_cmd_bld
                .arg("-i")
                .arg(mkvfile.clone())
                .arg(mp4file.clone());
        }
        let c1 = commands.add_node(ffmpeg_cmd);
        match first_transcode {
            None => {
                first_transcode = Some(c1);
            }
            Some(ft) => {
                commands.add_edge(ft, c1, ())?;
            }
        }
        let s3cmd_cmd = RefCell::new(Command::new("s3cmd"));
        {
            let mut s3cmd_cmd_bld = s3cmd_cmd.borrow_mut();
            s3cmd_cmd_bld
                .arg("put")
                .arg("-P")
                .arg(mp4file.clone())
                .arg(s3path.clone());
        }
        let (_, c2) = commands.add_child(c1, (), s3cmd_cmd);
        let delete_cmd = RefCell::new(Command::new("rm"));
        {
            let mut delete_cmd_bld = delete_cmd.borrow_mut();
            delete_cmd_bld.arg(mkvfile.clone());
        }
        commands.add_child(c2, (), delete_cmd);
    }
    return Ok(commands);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_commands() {
        use std::fs;
        use tempdir::TempDir;
        let config = Config {
            s3_screencasts_path: "s3://gradesta-web-static/screencasts/".to_string(),
            screencasts_base_url: "https://assets.gradesta.com/screencasts/".to_string(),
        };
        let tmp_dir_obj = TempDir::new("test_screencasts_dir").unwrap();
        let tmp_dir = tmp_dir_obj.path();
        let screencast1_path = tmp_dir.join("screencast1.mkv");
        let screencast2_path = tmp_dir.join("screencast2.mkv");
        fs::write(&screencast1_path, "foo").unwrap();
        fs::write(&screencast2_path, "foo").unwrap();
        let screencast1 = screencast1_path.to_string_lossy();
        let screencast2 = screencast2_path.to_string_lossy();
        let commands = transcode_and_upload(
            "My blog post {{<screencast \"video-id\">}} {{<screencast \"video-id-2\">}}",
            vec![&screencast1, &screencast2],
            &config,
        )
        .unwrap();

        assert_eq!(commands.edge_count(), 5);
        assert_eq!(commands.node_count(), 6);
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(0)).unwrap().borrow()),
            format!("Command {{ std: \"ffmpeg\" \"-i\" \"{0}/screencast1.mkv\" \"{0}/screencast1.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(1)).unwrap().borrow()),
            format!("Command {{ std: \"s3cmd\" \"put\" \"-P\" \"{0}/screencast1.mp4\" \"s3://gradesta-web-static/screencasts/video-id.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        assert_eq!(
            format!(
                "{:?}",
                commands
                    .node_weight(daggy::NodeIndex::new(2))
                    .unwrap()
                    .borrow()
            ),
            format!(
                "Command {{ std: \"rm\" \"{0}/screencast1.mkv\", kill_on_drop: false }}",
                tmp_dir.to_string_lossy()
            )
        );
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(3)).unwrap().borrow()),
            format!("Command {{ std: \"ffmpeg\" \"-i\" \"{0}/screencast2.mkv\" \"{0}/screencast2.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(4)).unwrap().borrow()),
            format!("Command {{ std: \"s3cmd\" \"put\" \"-P\" \"{0}/screencast2.mp4\" \"s3://gradesta-web-static/screencasts/video-id-2.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        commands
            .find_edge(daggy::NodeIndex::new(0), daggy::NodeIndex::new(1))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(1), daggy::NodeIndex::new(2))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(0), daggy::NodeIndex::new(3))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(3), daggy::NodeIndex::new(4))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(4), daggy::NodeIndex::new(5))
            .unwrap();
    }

    #[test]
    fn test_generate_commands_skipping_missing_input_files() {
        let config = Config {
            s3_screencasts_path: "s3://gradesta-web-static/screencasts/".to_string(),
            screencasts_base_url: "https://assets.gradesta.com/screencasts/".to_string(),
        };
        use std::fs;
        use tempdir::TempDir;
        let tmp_dir_obj = TempDir::new("test_screencasts_dir").unwrap();
        let tmp_dir = tmp_dir_obj.path();
        let screencast1_path = tmp_dir.join("screencast1.mkv");
        let screencast2_path = tmp_dir.join("screencast2.mkv");
        fs::write(&screencast1_path, "foo").unwrap();
        fs::write(&screencast2_path, "foo").unwrap();
        let screencast1 = screencast1_path.to_string_lossy();
        let screencast2 = screencast2_path.to_string_lossy();
        let commands = transcode_and_upload(
            "My blog post {{<screencast \"video-id\">}} {{<screencast \"video-id-2\">}} {{<screencast \"video-id-3\">}}",
            vec![
                "skip",
                &screencast1,
                &screencast2,
            ],
            &config,
        )
        .unwrap();

        assert_eq!(commands.edge_count(), 5);
        assert_eq!(commands.node_count(), 6);
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(0)).unwrap().borrow()),
            format!("Command {{ std: \"ffmpeg\" \"-i\" \"{0}/screencast1.mkv\" \"{0}/screencast1.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(1)).unwrap().borrow()),
            format!("Command {{ std: \"s3cmd\" \"put\" \"-P\" \"{0}/screencast1.mp4\" \"s3://gradesta-web-static/screencasts/video-id-2.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        assert_eq!(
            format!(
                "{:?}",
                commands
                    .node_weight(daggy::NodeIndex::new(2))
                    .unwrap()
                    .borrow()
            ),
            format!(
                "Command {{ std: \"rm\" \"{0}/screencast1.mkv\", kill_on_drop: false }}",
                tmp_dir.to_string_lossy()
            )
        );
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(3)).unwrap().borrow()),
            format!("Command {{ std: \"ffmpeg\" \"-i\" \"{0}/screencast2.mkv\" \"{0}/screencast2.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        assert_eq!(
            format!("{:?}", commands.node_weight(daggy::NodeIndex::new(4)).unwrap().borrow()),
            format!("Command {{ std: \"s3cmd\" \"put\" \"-P\" \"{0}/screencast2.mp4\" \"s3://gradesta-web-static/screencasts/video-id-3.mp4\", kill_on_drop: false }}", tmp_dir.to_string_lossy()));
        commands
            .find_edge(daggy::NodeIndex::new(0), daggy::NodeIndex::new(1))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(1), daggy::NodeIndex::new(2))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(0), daggy::NodeIndex::new(3))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(3), daggy::NodeIndex::new(4))
            .unwrap();
        commands
            .find_edge(daggy::NodeIndex::new(4), daggy::NodeIndex::new(5))
            .unwrap();
    }

    #[test]
    fn test_generate_commands_too_few_inputs() {
        let config = Config {
            s3_screencasts_path: "s3://gradesta-web-static/screencasts/".to_string(),
            screencasts_base_url: "https://assets.gradesta.com/screencasts/".to_string(),
        };
        use std::fs;
        use tempdir::TempDir;
        let tmp_dir_obj = TempDir::new("test_screencasts_dir").unwrap();
        let tmp_dir = tmp_dir_obj.path();
        let screencast1_path = tmp_dir.join("screencast1.mkv");
        fs::write(&screencast1_path, "foo").unwrap();
        let screencast1 = screencast1_path.to_string_lossy();

        let commands = transcode_and_upload(
            "My blog post {{<screencast \"video-id\">}} {{<screencast \"video-id\">}}",
            vec![&screencast1],
            &config,
        );
        match commands {
            Ok(_) => assert!(false),
            Err(e) =>
                assert_eq!(e.to_string(), "There are 2 screencasts refered to in the blog post but 1 were passed. Please pass one video file per screencast tag.")
        }
    }
}
