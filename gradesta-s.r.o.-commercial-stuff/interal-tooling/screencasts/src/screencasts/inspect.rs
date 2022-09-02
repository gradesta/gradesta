use super::parsing::extract_screencast_tags;
use std::fs;

pub async fn inspect(blog_post_path: &str) -> anyhow::Result<()> {
    let blogpost = fs::read_to_string(blog_post_path).unwrap();
    let (_, screencasts) = extract_screencast_tags(&blogpost)?;
    for screencast in &screencasts {
        println!("{}", screencast.id);
    };
    println!("Found {} screencasts.", screencasts.len());
    Ok(())
}
