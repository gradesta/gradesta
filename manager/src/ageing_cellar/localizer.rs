use lazy_static::lazy_static;
use tokio::sync::{Mutex};
use std::sync::Arc;
use std::convert::From;

use fluent::concurrent::FluentBundle;
use fluent::{FluentValue, FluentResource, FluentArgs};

// Used to provide a locale for the bundle.
use unic_langid::LanguageIdentifier;


lazy_static! {
    static ref FLUENT_BUNDLE: Arc<Mutex<FluentBundle<FluentResource>>> = {

        // Load locale from LANG env var into string
        let lang = std::env::var("LANG").unwrap_or("en-US".to_string());
        // Remove the encoding (if any) from the locale (this is whatever is followed by a dot)
        let lang = lang.split('.').next().unwrap();
        let langid: LanguageIdentifier = lang.parse()
            .expect(format!("Parsing locale failed {}", lang).as_str());
        let mut bundle = FluentBundle::new(vec![langid.clone()]);


        let languages = vec![
            ("en", include_str!("../../locale/en.ftl")),
            ("cs", include_str!("../../locale/cs.ftl")),
        ];

        // Loop through languages and add them as resources to the bundle.
        for language in languages {
            if language.0 == langid.language.as_str() {
                let res = FluentResource::try_new(language.1.to_string())
                    .expect("Parsing failed.");
                bundle.add_resource(res).expect("Adding resource failed.");
            }
        }

        Arc::new(Mutex::new(bundle))

    };
}

async fn get_localized_string<'a> (key: &str, args: Option<&FluentArgs<'a>>) -> String {
    let bundle = FLUENT_BUNDLE.lock().await;
    let msg = bundle.get_message(key).expect("Message not found.");
    let pattern = msg.value.expect("Message has no value.");
    let mut errors = vec![];
    let value = bundle.format_pattern(pattern, args, &mut errors);
    value.to_string()
}

/// Get the localized string for a given key.
pub async fn l (key: &str) -> String {
    get_localized_string(key, None).await
}

/// Get the localized string for a given key passing one argument.
pub async fn l1 <'a, A> (key: &str, arg: &str, value: A) -> String
    where FluentValue<'a>: From<A>
    {
    let mut args = FluentArgs::new();
    args.add(arg, FluentValue::from(value));
    get_localized_string(key, Some(&args)).await
}

/// Get the localized string for a given key passing two arguments.
pub async fn l2 <'a, A, B> (key: &str, arg1: &str, value1: A, arg2: &str, value2: B) -> String
    where FluentValue<'a>: From<A> + From<B>
    {
    let mut args = FluentArgs::new();
    args.add(arg1, FluentValue::from(value1));
    args.add(arg2, FluentValue::from(value2));
    get_localized_string(key, Some(&args)).await
}

/// Get the localized string for a given key passing three arguments.
pub async fn l3 <'a, A, B, C> (key: &str, arg1: &str, value1: A, arg2: &str, value2: B, arg3: &str, value3: C) -> String
    where FluentValue<'a>: From<A> + From<B> + From<C>
    {
    let mut args = FluentArgs::new();
    args.add(arg1, FluentValue::from(value1));
    args.add(arg2, FluentValue::from(value2));
    args.add(arg3, FluentValue::from(value3));
    get_localized_string(key, Some(&args)).await
}

// Test
#[tokio::test]
async fn test_localizer() {
    let hello_world = l("test_case_simple").await;
    assert_eq!(hello_world, "Hello world!");

    let localized_string = l1("test_case_intro", "name", "John").await;
    assert_eq!(localized_string, "Welcome, \u{2068}John\u{2069}.");

    let localized_plural = l1("test_case_plural", "num", 3).await;
    assert_eq!(localized_plural, "You have a few new messages.");

    let two_params = l2("test_case_two_params", "one", "John", "two", 30).await;
    assert_eq!(two_params, "1. \u{2068}John\u{2069} 2. \u{2068}30\u{2069}");

    let three_params = l3("test_case_three_params", "one", "John", "two", 30, "three", "Doe").await;
    assert_eq!(three_params, "1. \u{2068}John\u{2069} 2. \u{2068}30\u{2069} 3. \u{2068}Doe\u{2069}");
}
