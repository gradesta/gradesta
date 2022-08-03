use super::parsing::{extract_screencast_tags};
use std::path::{Path};
use std::error::{Error};
use daggy::Dag;

type Command = Vec<String>;

/// Takes the contents of a blog post and returns a list of lists of external commands to run.
/// These lists can be run in paralel.
///
pub fn transcode_and_upload<'a>(blogpost: &'a str, video_files: Vec<&str>) -> Result<Dag<Command, ()>, Box<dyn Error + 'a>>
{
    let (_, screencasts) = extract_screencast_tags(blogpost)?;
    if video_files.len() != screencasts.len() {
        return Err(format!("There are {} screencasts refered to in the blog post but {} were passed. Please pass one video file per screencast tag.",
                           screencasts.len(),
                           video_files.len()
        ).into())
    }
    let mut commands: Dag<Command, ()> = Dag::new();
    let mut c1 = commands.add_node(vec!["echo", "Starting uploads"].into_iter().map(|s| s.to_string()).collect()); // First command
    let mut i = 0;
    for screencast in screencasts {
        let mkvfile = video_files[i];
        let mp4file_path = Path::new(video_files[i]).with_extension("mp4");
        let path_encoding_error: Box<dyn Error> = format!("Invalid character in video file path {}.", video_files[i]).into();
        let mp4file = mp4file_path.to_str().ok_or(path_encoding_error)?;
        let s3path = format!("s3://gradesta-web-static/screencasts/{}.mp4", screencast.id);
        let (_, c2) = commands.add_child(c1, (), vec!["ffmpeg", "-i", mkvfile, mp4file].into_iter().map(|s| s.to_string()).collect());
        commands.add_child(c2, (), vec!["s3cmd", "put", "-P", mp4file, &s3path].into_iter().map(|s| s.to_string()).collect());
        i += 1;
    };
    return Ok(commands);
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_commands() {
        let commands = transcode_and_upload("My blog post {{<screencast \"video-id\">}}", vec!["/home/user/Videos/screencast1.mkv"]).unwrap();
        assert_eq!(format!("{:?}", commands), "Dag { graph: Graph { Ty: \"Directed\", node_count: 3, edge_count: 2, edges: (0, 1), (1, 2), node weights: {0: [\"echo\", \"Starting uploads\"], 1: [\"ffmpeg\", \"-i\", \"/home/user/Videos/screencast1.mkv\", \"/home/user/Videos/screencast1.mp4\"], 2: [\"s3cmd\", \"put\", \"-P\", \"/home/user/Videos/screencast1.mp4\", \"s3://gradesta-web-static/screencasts/video-id.mp4\"]} }, cycle_state: DfsSpace { dfs: Dfs { stack: [], discovered: FixedBitSet { data: [], length: 0 } } } }");

        let commands = transcode_and_upload("My blog post {{<screencast \"video-id\">}} {{<screencast \"video-id\">}}", vec!["/home/user/Videos/screencast1.mkv"]);
        match commands {
            Ok(_) => assert!(false),
            Err(e) =>
                assert_eq!(e.to_string(), "There are 2 screencasts refered to in the blog post but 1 were passed. Please pass one video file per screencast tag.")
        }
    }
}
