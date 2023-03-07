---
title: "Localization of the manager"
date: 2023-03-07
featureimage: https://assets.gradesta.com/gradesta/img/dalle2-clock-and-coins.png
author: timothy hobbs <tim@gradesta.com>
draft: true
---

Part 1: Researching localization methods
------------------------------------------------

I've decided to start working at a co-working space where I cannot talk. I still have an office at home so I will record audio for some of the screencasts, but for this screencast I will be silent. I also found talking to be strangely tireing, and sometimes I have struggled to gather the engergy to talk so here we are, silentcasting.

I need to come up with a good method for localizing the manager. I want to figure this out early because I don't want to do a whole bunch of refactoring if my localization method ends up completly changing how strings are managed in the manager. I guess I've been generally unhappy with the methods that I have used in the past. I always believed in the idea of separating presentation and business logic yet so much text ends up being put into strings directly in the program, especially for CLI apps like this manager. I think that having a great UI that can be improved by people who are not proficient in rust is a must and while I dissagree with the whole mindset that you have to be some computer wiz kid to edit a string in an unknown language it seems like a pretty prevelant opinion. It seems weird to me that localization schemse so often are based on the idea that "we'll hard code the English and let other people write the translations." Why should the English be written by a computer programmer while the translations are written by someone focusing on the language? Often times I have experienced that the Czech versions of computer interfaces were supperior to the English versions for the very reason that the Czech person who wrote the translation was just generally better at explaining things...

First off, I need to do a bit of a survey of localization methods in Rust. I have never done localizaiont in rust so I am in the dark here.

Before I start though, I want to take a moment to gather some examples of strings that exist in the (rather small so far), manager source code.

```
        let dangling_sockets = organize_sockets_dir(sockets_dir, &collections::HashSet::new())?;
        if dangling_sockets.len() > 0 {
            Err(anyhow!("Could not start because the following gradesta service sockets are still in use: \n{}", display_dangling_sockets(dangling_sockets)?))
        } else {
            Ok(())
        }
```

```
    match system_info.process(i32pid) {
        Some(process) => {
            return Ok(format!("    {} - {}\n", i32pid, process.name()));
        }
        None => return Ok(format!("    {} - no longer running\n", i32pid)),
    };
}
```

```
        Err(anyhow!(
            "Error unlocking lock file {:?}: {}",
            lockfile.as_os_str(),
```

```
    println!("Launching gradesta manager.");
```

```
            println!(
                "Will watch for clients binding sockets in {:?}",
                sockets_dir
            );
```

Now just glancing through the codebase so far, it seems that a little over half of the strings are error messages. I have good reason to believe that error messages might be better off left untranslated. Anyone who finds themselves searching the internet for other poor souls trying to decifer an inscrutable error message in Czech quickly finds themselves switching the interface language over to English in order to improve searchablity. Some projects use error codes to try to alleviate this issue. Sure the error might be in Czech wich is a minority language, but the error code is universal and searchable. One important thing to think about, though is whether the error codes are truely unique. A code like `E133` is totally unsearchable, maybe it can be searched for if you search for `Gradesta E133` but just `E133` is just not a good search term. Perhaps `GRADESTA_ERROR_133` would be better, but it can add to the verbosity of the error message. Perhaps it would be better to write out all errors in English and then add a localized "Explanation" to each error. That still leaves the question as to whether strings should be hard-coded into the source code at all. I guess the fact that hard coded English strings make the source far more readable is a strong argument for hard-coding where-as the separation of presentation and logic is weaker. I guess I'll stick with hard coding of English.

I guess the other aspect of localization is the issue of dates and numbers. There are two types of dates, sane dates and American dates. Lets just not localize dates. But numbers are different, numbers can be written:

1.2  1,000.2
1,2  1 000,2
1,2  1.000,2

As far as I recall, and this just differs by culture without any of the schemes being objectively better. It is probably a good idea to localize numbers.

Another aspect to consider is pluralization and inflection. If you have a string like "There are <n> dogs." You may need to localize it in multuple forms: `There is one dog.` `There are 2 dogs.` And in some languages there are even more forms, like in Czech we have different pluralization for groups larger than 3 in some cases. This pluralization is necessary in English as well as other languages. Its a bit weird to use an if statement in English and then have that logic differ by language. I guess it's another point in terms of separating presentation and logic.

Inflection is the case in which you are generating an email and you write `Dear Ana`. In Czech that's `Vážená Ano`. You have to change the name to another case. If we have `Ana` in a name variable, that means we are actually modifying the presentation of a string! This is a bit like the localization of numbers. Lets just ignore this problem though, as it seems like it is not that important for a CLI app like gradesta's manager.

Another aspect that is quite important is state. Does the internationalization system add more state that needs to be passed around? I hate it when I have to have some "formatter" object or "lang" object which must be passed to every function that has to print something out. I guess this is probably a non-starter for any lib that does this. I know global state is evil, but dependency injection can get a little over the top as well.

So according to [this logrocket article](https://blog.logrocket.com/rust-internationalization-localization-and-translation/) we can choose between:

1. [`gettext`](https://docs.rs/gettext-rs/latest/gettextrs/) which I know well
2. [`project fluent`](https://github.com/projectfluent/fluent-rs)  which I've never heard of
3. [`ICU Message Format`](https://unicode-org.github.io/icu/userguide/format_parse/messages/) which is also new to me

The article also lists a lot of other libs.

```
TASK: figure out manager localization
TASK_ID: 99ce23b59c7f33b4b63e442443fd1f88
CREATED: 2022-09-01 18:49
ESTIMATED_TIME: U2 W4
MILESTONES: mvp
```

{{<screencast "2023-03-07-4f7140d2-56cd-42a8-96a6-11ccd4f650ff" "99ce23b59c7f33b4b63e442443fd1f88">}}

Part 2: Selecting a sollution
----------------------------------

{{<screencast "2023-03-07-19a422c5-d619-4666-bb14-e05d6fc5c63e" "99ce23b59c7f33b4b63e442443fd1f88">}**

**Fluent**

Looking at the [fluent docs](https://crates.io/crates/fluent) it seems that it fails to not require us to pass around some translator object.

```
use fluent::{FluentBundle, FluentResource};
use unic_langid::langid;

fn main() {
    let ftl_string = "hello-world = Hello, world!".to_owned();
    let res = FluentResource::try_new(ftl_string)
        .expect("Failed to parse an FTL string.");

    let langid_en = langid!("en-US");
    let mut bundle = FluentBundle::new(vec![langid_en]);

    bundle.add_resource(&res)
        .expect("Failed to add FTL resources to the bundle.");

    let msg = bundle.get_message("hello-world")
        .expect("Message doesn't exist.");
    let mut errors = vec![];
    let pattern = msg.value
        .expect("Message has no value.");
    let value = bundle.format_pattern(&pattern, None, &mut errors);

    assert_eq!(&value, "Hello, world!");
}
```

Everything I see here is `bundle.**. While this is technically quite a good thing, in practice this is a huge hassle. I'm going to skip fluent for this reason.

**rust-icu**

Looking at [rust-icu](https://crates.io/crates/rust_icu** I'm not seeing great deal of documentation on what it looks like in practice. Furthermore, it is a native binding that requires weird build stuff. I'd like to avoid making it harder to build the gradesta manager. Perhaps I should also take that into account and prefer libraries that don't require external non-native deps.

That same argument, unfortunately, would also apply got gettext...

**twine**

This one looks interesting though there is some state passing. 

```
fn main() {
    // use "" if there is no localization
    let lang = Lang::Fr("be");

    // will output "Ruiner le nom d'un groupe en le traduisant en français"
    t!(app_ruin_the_band => lang);

    // using formatted arguments, this will output "73 %"
    t!(format_percentage, 73.02f32 => lang);
}
```

One facinating fact is this: "All translation keys must have all the languages of all the keys. For example, if all your keys have translations for en and fr, if one key has only en, it will fail to compile.". Awsome and problematic at the same time. Translations are never out of date, but you cannot add a new string without speaking all the supported languages? I'm a bit confused though because the next sentences are "Localized translation can be provided and will be used if available. Otherwise it will fallback to the default translation for that language." Sow will it fallback to the default or fail to compile? Which is it?

This libarary seems simple and somehow I like it. One potential downside is that it appears to not be very popular. I'm also not seeing how it does pluralization yet.

It is not clear to me how this project relates to ruby's [twine](https://github.com/scelis/twine) which also does translations. The file examples look slightly different, I think they are the same though. One thing I really like here is that unlike the `po` `mo` situation with gettext we arent autogenerating translation files. My experience is that the autogenerated po files create really nasty looking git diffs.

Unfortunately it appears that the twine format [is not able to handle plurals at all](https://github.com/scelis/twine/issues/46**. I guess that's a non-starter.

**Revisiting fluent**

So this leads us back to fluent. It appears that fluent is pure rust so there shouldn't be any weird compilation shinanigans. It also uses a sane file format that won't change in random huge ways each time line numbers get shifted like with PO files. This will reduce the weirdness of the git diffs. The only thing that I don't like is the need to pass the `bundle` object around. 

