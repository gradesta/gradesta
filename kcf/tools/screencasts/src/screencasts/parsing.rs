/*
/ This is code used to parse hugo blog posts and extract information about coding screencasts.
*/

/*
TASK: Nom doesn't give good errors, swithch to https://github.com/Marwes/combine
TASK_ID: a7cf535496c5a1437f341492e988d17d
CREATED: 2022-09-04 15:08
ESTIMATED_TIME: W4
MILESTONES: nitpicks
*/

use super::frontmatter::FrontMatter;
use super::screencast::Screencast;

use nom::{
    branch::alt,
    bytes::complete::{is_a, tag, take, take_until, take_while},
    character::complete::char,
    combinator::{map, opt, rest},
    error::ParseError,
    multi::fold_many0,
    sequence::{preceded, terminated, tuple, separated_pair},
    IResult,
};

use anyhow::anyhow;

/*
/ Post content is ignored, we're only interested in screencast tags.
*/
fn content<'a>(i: &'a str) -> IResult<&'a str, &str> {
    alt((take_until("{{"), rest))(i)
}

fn whitespace<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    let chars = " \t\r\n";

    // nom combinators like `take_while` return a function. That function is the
    // parser,to which we can pass the input
    take_while(move |c| chars.contains(c))(i)
}

fn screencast_tasks<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Vec<&str>, E> {
    preceded(
        char('"'),
        terminated(
            map(take_until("\""), |tasks: &str| tasks.split(" ").collect::<Vec<&str>>()),
            char('"'),
        ),
    )(i)
}

fn screencast_id<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, &'a str, E> {
    preceded(
        char('"'),
        terminated(
            take_until("\""),
            char('"'),
        ),
    )(i)
}

fn screencast_tag<'a>(i: &'a str) -> IResult<&'a str, Screencast> {
    preceded(
        char('<'),
        terminated(
            preceded(
                whitespace,
                preceded(is_a("screencast"), preceded(whitespace, map(separated_pair(screencast_id, whitespace, opt(screencast_tasks)), |(id, tasks)| {
                    let mut screencast = Screencast::new(id.to_string());
                    match tasks {
                        Some(ts) => screencast.tasks = ts.iter().map(|t| t.to_string()).collect(),
                        None => ()
                    }
                    screencast
                }))),
            ),
            char('>'),
        ),
    )(i)
}

fn hugo_tag<'a>(i: &'a str) -> IResult<&'a str, Option<Screencast>> {
    preceded(
        is_a("{{"),
        terminated(opt(screencast_tag), tuple((take_until("}}"), take(2usize)))),
    )(i)
}

pub fn extract_screencast_tags<'a>(i: &'a str) -> anyhow::Result<(&str, Vec<Screencast>)> {
    match fold_many0(
        preceded(content, terminated(hugo_tag, content)),
        || vec![],
        |mut acc, item| match item {
            Some(screencast) => {
                acc.push(screencast);
                acc
            }
            None => acc,
        },
    )(i)
    {
        Ok(ss) => Ok(ss),
        Err(e) => Err(anyhow!("{}", e)),
    }
}

pub fn extract_frontmatter<'a>(i: &'a str) -> IResult<&str, FrontMatter> {
    preceded(
        whitespace,
        preceded(
            tag("---"),
            map(take_until("---"), |frontmatter: &str| {
                serde_yaml::from_str::<FrontMatter>(frontmatter).unwrap()
            }),
        ),
    )(i)
}

pub fn load_screencasts_from_blogpost(blogpost: &str) -> anyhow::Result<Vec<Screencast>> {
    let (remainder, frontmatter) = match extract_frontmatter(blogpost) {
        Ok((r, f)) => Ok((r, f)),
        Err(e) => Err(anyhow!("{:?}", e)),
    }?;
    let (_, mut screencasts) = extract_screencast_tags(remainder)?;

    for i in 0..screencasts.len() {
        screencasts[i].authors = frontmatter.authors.clone();
    }
    Ok(screencasts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_content() {
        let (remainder, parsed_content) = content("foo bar baz").unwrap();
        assert_eq!(remainder, "");
        assert_eq!(parsed_content, "foo bar baz");

        let (remainder, parsed_content) = content("").unwrap();
        assert_eq!(remainder, "");
        assert_eq!(parsed_content, "");

        let (remainder, parsed_content) = content("foo {{bar}}").unwrap();
        assert_eq!(remainder, "{{bar}}");
        assert_eq!(parsed_content, "foo ");
    }

    #[test]
    fn test_read_screencast_tag() {
        let (remainder, parsed_tag) = screencast_tag("<screencast \"lol\">}} foo").unwrap();
        assert_eq!(remainder, "}} foo");
        assert_eq!(parsed_tag, Screencast::new("lol".to_string()));

        let (remainder, parsed_tag) = screencast_tag("<screencast \"lol\" \"task1 task2\">}} foo").unwrap();
        assert_eq!(remainder, "}} foo");
        let mut sc = Screencast::new("lol".to_string());
        sc.tasks = vec!["task1".to_string(), "task2".to_string()];
        assert_eq!(parsed_tag, sc);

    }

    #[test]
    fn test_read_hugo_tag() {
        let (remainder, parsed_tag) = hugo_tag("{{<nothing \"\">}} foo").unwrap();
        assert_eq!(remainder, " foo");
        assert_eq!(parsed_tag, None);

        let (remainder, parsed_tag) = hugo_tag("{{<screencast \"bar\">}} foo").unwrap();
        assert_eq!(remainder, " foo");
        assert_eq!(parsed_tag, Some(Screencast::new("bar".to_string())));
    }

    #[test]
    fn test_extract_screencast_tags() {
        let (remainder, screencasts) =
            extract_screencast_tags("Foo {{<screencast \"abc\">}} {{<screencast \"def\">}}")
                .unwrap();
        assert_eq!(remainder, "");
        assert_eq!(
            vec![
                Screencast::new("abc".to_string()),
                Screencast::new("def".to_string())
            ],
            screencasts
        );
    }

    #[test]
    fn test_extract_frontmatter() {
        let (remainder, frontmatter) =
            match extract_frontmatter("---\nauthors: [\"Timothy Hobbs <timothyhobbs@seznam.cz>\"]\ndate: 2022-08-23\ntitle: Foo ---\nFoo {{<screencast \"abc\">}} {{<screencast \"def\">}}")
            {
                Err(e) => panic!("{}", e),
                Ok((r, fm)) => (r, fm)
            };
        assert_eq!(
            remainder,
            "---\nFoo {{<screencast \"abc\">}} {{<screencast \"def\">}}"
        );
        assert_eq!(
            vec!["Timothy Hobbs <timothyhobbs@seznam.cz>".to_string()],
            frontmatter.authors
        );
        assert_eq!("Foo".to_string(), frontmatter.title);
    }
}
