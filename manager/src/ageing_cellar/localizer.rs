use lazy_static::lazy_static;
use tokio::sync::{Mutex};
use std::sync::Arc;

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

pub async fn get_localized_string<'a> (key: &str, args: Option<&FluentArgs<'a>>) -> String {
    let bundle = FLUENT_BUNDLE.lock().await;
    let msg = bundle.get_message(key).expect("Message not found.");
    let pattern = msg.value.expect("Message has no value.");
    let mut errors = vec![];
    let value = bundle.format_pattern(pattern, args, &mut errors);
    value.to_string()
}

// Test
#[tokio::test]
async fn test_localizer() {
    let mut args = FluentArgs::new();
    args.add("name", FluentValue::from("John"));
    let localized_string = get_localized_string("test_case_intro", Some(&args)).await;
    assert_eq!(localized_string, "Welcome, \u{2068}John\u{2069}.");

    let mut plural_args = FluentArgs::new();
    plural_args.add("num", FluentValue::from(3));

    let localized_plural = get_localized_string("test_case_plural", Some(&plural_args)).await;
    assert_eq!(localized_plural, "You have a few new messages.");
}
