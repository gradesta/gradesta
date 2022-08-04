use super::parsing::extract_screencast_tags;
use daggy::Dag;
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
) -> Result<Dag<RefCell<Command>, ()>, Box<dyn Error + 'a>> {
    let (_, screencasts) = extract_screencast_tags(blogpost)?;
    if video_files.len() != screencasts.len() {
        return Err(format!("There are {} screencasts refered to in the blog post but {} were passed. Please pass one video file per screencast tag.",
                           screencasts.len(),
                           video_files.len()
        ).into());
    }
    let mut commands: Dag<RefCell<Command>, ()> = Dag::new();
    let mut i = 0;
    for screencast in screencasts {
        let mkvfile = video_files[i];
        let mp4file_path = Path::new(video_files[i]).with_extension("mp4");
        let path_encoding_error: Box<dyn Error> =
            format!("Invalid character in video file path {}.", video_files[i]).into();
        let mp4file = mp4file_path.to_str().ok_or(path_encoding_error)?;
        let s3path = format!("s3://gradesta-web-static/screencasts/{}.mp4", screencast.id);
        let ffmpeg_cmd = RefCell::new(Command::new("ffmpeg"));
        {
            let mut ffmpeg_cmd_bld = ffmpeg_cmd.borrow_mut();
            ffmpeg_cmd_bld
                .arg("-i")
                .arg(mkvfile.clone())
                .arg(mp4file.clone());
        }
        let c1 = commands.add_node(ffmpeg_cmd);
        let s3cmd_cmd = RefCell::new(Command::new("s3cmd"));
        {
            let mut s3cmd_cmd_bld = s3cmd_cmd.borrow_mut();
            s3cmd_cmd_bld
                .arg("put")
                .arg("-P")
                .arg(mp4file.clone())
                .arg(s3path.clone());
        }
        commands.add_child(c1, (), s3cmd_cmd);
        i += 1;
    }
    return Ok(commands);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_commands() {
        let commands = transcode_and_upload(
            "My blog post {{<screencast \"video-id\">}} {{<screencast \"video-id-2\">}}",
            vec![
                "/home/user/Videos/screencast1.mkv",
                "/home/user/Videos/screencast2.mkv",
            ],
        )
        .unwrap();
        assert_eq!(format!("{:?}", commands), "Dag { graph: Graph { Ty: \"Directed\", node_count: 4, edge_count: 2, edges: (0, 1), (2, 3), node weights: {0: RefCell { value: Command { std: \"ffmpeg\" \"-i\" \"/home/user/Videos/screencast1.mkv\" \"/home/user/Videos/screencast1.mp4\", kill_on_drop: false } }, 1: RefCell { value: Command { std: \"s3cmd\" \"put\" \"-P\" \"/home/user/Videos/screencast1.mp4\" \"s3://gradesta-web-static/screencasts/video-id.mp4\", kill_on_drop: false } }, 2: RefCell { value: Command { std: \"ffmpeg\" \"-i\" \"/home/user/Videos/screencast2.mkv\" \"/home/user/Videos/screencast2.mp4\", kill_on_drop: false } }, 3: RefCell { value: Command { std: \"s3cmd\" \"put\" \"-P\" \"/home/user/Videos/screencast2.mp4\" \"s3://gradesta-web-static/screencasts/video-id-2.mp4\", kill_on_drop: false } }} }, cycle_state: DfsSpace { dfs: Dfs { stack: [], discovered: FixedBitSet { data: [], length: 0 } } } }");

        let commands = transcode_and_upload(
            "My blog post {{<screencast \"video-id\">}} {{<screencast \"video-id\">}}",
            vec!["/home/user/Videos/screencast1.mkv"],
        );
        match commands {
            Ok(_) => assert!(false),
            Err(e) =>
                assert_eq!(e.to_string(), "There are 2 screencasts refered to in the blog post but 1 were passed. Please pass one video file per screencast tag.")
        }
    }
}
