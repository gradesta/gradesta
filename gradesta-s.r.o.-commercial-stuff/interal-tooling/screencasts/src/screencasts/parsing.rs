/*
/ This is code used to parse hugo blog posts and extract information about coding screencasts.
*/
use nom::{
    branch::alt,
    bytes::complete::{is_a, take, take_until, take_while},
    character::complete::char,
    combinator::{map, opt, rest},
    error::ParseError,
    multi::fold_many0,
    sequence::{preceded, terminated, tuple},
    IResult,
};

use anyhow::anyhow;

#[derive(PartialEq, Eq, Debug)]
pub struct Screencast<'a> {
    pub id: &'a str,
    //authors: Vec<String>,
}

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

fn screencast_id<'a, E: ParseError<&'a str>>(i: &'a str) -> IResult<&'a str, Screencast, E> {
    preceded(
        char('"'),
        terminated(map(take_until("\""), |id| Screencast { id: id }), char('"')),
    )(i)
}

fn screencast_tag<'a>(i: &'a str) -> IResult<&'a str, Screencast<'a>> {
    preceded(
        char('<'),
        terminated(
            preceded(
                whitespace,
                preceded(is_a("screencast"), preceded(whitespace, screencast_id)),
            ),
            char('>'),
        ),
    )(i)
}

fn hugo_tag<'a>(i: &'a str) -> IResult<&'a str, Option<Screencast<'a>>> {
    preceded(
        is_a("{{"),
        terminated(opt(screencast_tag), tuple((take_until("}}"), take(2usize)))),
    )(i)
}

pub fn extract_screencast_tags<'a>(i: &'a str) -> anyhow::Result<(&str, Vec<Screencast<'a>>)> {
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
    )(i) {
        Ok(ss) => Ok(ss),
        Err(e) => Err(anyhow!("{}", e)),
    }
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
        assert_eq!(parsed_tag, Screencast { id: "lol" });
    }

    #[test]
    fn test_read_hugo_tag() {
        let (remainder, parsed_tag) = hugo_tag("{{<nothing \"\">}} foo").unwrap();
        assert_eq!(remainder, " foo");
        assert_eq!(parsed_tag, None);

        let (remainder, parsed_tag) = hugo_tag("{{<screencast \"bar\">}} foo").unwrap();
        assert_eq!(remainder, " foo");
        assert_eq!(parsed_tag, Some(Screencast { id: "bar" }));
    }

    #[test]
    fn test_extract_screencast_tags() {
        let (remainder, screencasts) =
            extract_screencast_tags("Foo {{<screencast \"abc\">}} {{<screencast \"def\">}}")
                .unwrap();
        assert_eq!(remainder, "");
        assert_eq!(
            vec![Screencast { id: "abc" }, Screencast { id: "def" }],
            screencasts
        );
    }
}
