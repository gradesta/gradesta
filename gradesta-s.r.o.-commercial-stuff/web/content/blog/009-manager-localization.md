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

Part 2: Selecting a solution
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


Part 3: More examination of fluent
-----------------------------------------

{{<screencast "2023-03-15-bfdc889b-dfdd-4fa1-9819-1657f30d8ef2">}}

I found the following in the fluent docs:

```
Ergonomics & Higher Level APIs

Reading the example, you may notice how verbose it feels. Many core methods are fallible, others accumulate errors, and there are intermediate structures used in operations.

This is intentional as it serves as building blocks for variety of different scenarios allowing implementations to handle errors, cache and optimize results.

At the moment it is expected that users will use the fluent-bundle crate directly, while the ecosystem matures and higher level APIs are being developed.
```

This suggests to me that perhaps rust's fluent API is not particularly stable. This gives me pause. It really does look not-fun to work with :/

I decided to take a look at fluent's [dependency graph](https://github.com/projectfluent/fluent-rs/network/dependents) on github. Apparently it is used by 3.5 thousand repos. Not bad, probably is usable then (unless those are all dups...) 

Delving in I found [this commit](https://github.com/crypt0kitty/czkawka/commit/77a48ca6aa1b1b34908a98ff97c6e05d17457df3) in some random repo. This repo isn't using fluent directly, but rather is using [i18n-embed].

What the end up doing is actually [defining](https://github.com/crypt0kitty/czkawka/commit/77a48ca6aa1b1b34908a98ff97c6e05d17457df3?diff=unified#diff-cb9d429954f216ede69c4b8d3d36b3c950a22865e103d319b6a9bb854c5c36a2) an `fl!` macro for themselves to make the fluent calls less verbose. Then their diff looks like:


```
                    format!("Very High {}", *h)
                    format!("{} {}", fl!("core_similarity_very_high"), *h)
```

So the fluent system is replacing easy to read english language strings with these string slugs. Certainly makes the codebase harder to read and a bit more verbose. But hm. I'll have to experience it to see if it works out. I see a lot of problems with this. Less context for co-pilot, less context for the coder. Perhaps I'd do something like:

```
-                   format!("Very High {}", *h)
+                   format!("{} {}", fl!("core_similarity_very_high" /*Very High*/), *h)
```


I kind of like this, and the macro appears to be MIT license. I think I'll just end up copying it.

Acutally, unfortunately, it turns out that this isn't a very good system. It locks word order/message formatting into the source code. This isn't really good because some languages have different natural word order.

```
-                    entry_info.set_text(format!("Found {} broken files.", broken_files_number).as_str());
+                    entry_info.set_text(format!("{} {} {}.", fl!("compute_found"), broken_files_number, fl!("compute_broken_files")).as_str());
```

here if a language's natural word order was something like "Broken files {} found" rather than "Found {} broken files" it would be impossible for the translator to put in the correct word order.  So this macro method is probably a non-starter... Off to looking at more fluent examples to see if there's a better way...

While doing this I cam accross an interesting repo description: "If the broad light of day could be let in upon men’s actions, it would purify them as the sun disinfects. " [source](https://github.com/garthtrickett/light). It turns out that the only reason this repo is showing up in the dependency graph is that it contains a copy of tauri which apparently uses fluent for localization. Other repos in the "used by" list appear not to import fluent at all.


I was able to find [another project using fluent](https://github.com/fastn-stack/fastn/blob/58f5307761167e5ded52cab4b464668ea0985eb7/fastn-core/i18n/en/translation.ftl#L25) and this one is doing so in a way that encodes word order in the translations.

From the ftl file:

```
out-dated-body = The { $lang } document was last modified on { $last-modified-on }. Since then, the { $primary-lang } version has the following changes.
```

But the way of sending these context variables to fluent is downright strange. There is this [large source file](https://github.com/fastn-stack/fastn/blob/aa237f510b1bc3bc05990587ecec75f0218d2c37/fastn-core/src/library/fastn_dot_ftd.rs) in wich every single message is loaded into variables, and the variables are loaded by passing all possible context vars to all possible strings, regardless of whether they need that context :O

```
        out_dated_body = fastn_core::i18n::translation::search(
            &lang,
            &primary_lang,
            "out-dated-body",
            &current_document_last_modified_on
        ),
        out_dated_heading = fastn_core::i18n::translation::search(
            &lang,
            &primary_lang,
            "out-dated-heading",
            &current_document_last_modified_on
        ),
        show_latest_version = fastn_core::i18n::translation::search(
            &lang,
            &primary_lang,
            "show-latest-version",
            &current_document_last_modified_on
        ),
```

{{<screencast "2023-03-22-10ba1845-bcb4-47c8-b8dd-582b43d15154" "99ce23b59c7f33b4b63e442443fd1f88">}}

So after looking at things closely, fluent does have [full pluralization support](https://mozilla-l10n.github.io/localizer-documentation/tools/fluent/basic_syntax.html#selectors-and-plurals). Now we just need to deal with passing the `bundle` arround. I guess that logging and localization are kind of special cases in which global variables make sense.

So I spent a lot of time trying to figure out how to do a global fluent bundle variable. [According to stackoverflow](https://stackoverflow.com/questions/19605132/is-it-possible-to-use-global-variables-in-rust), I should be storing it in a Mutex in a `lazy_static`. However, when I try that I get an error (even with Arc):

```
error[E0277]: `(dyn Any + 'static)` cannot be sent between threads safely
  --> src/ageing_cellar/localizer.rs:10:1
   |
10 | / lazy_static! {
11 | |     static ref FLUENT_BUNDLE: Arc<Mutex<FluentBundle<FluentResource>>> = {
12 | |
13 | |         let langid_en: LanguageIdentifier = "en-US".parse().expect("Parsing failed");
...  |
31 | |     };
32 | | }
   | |_^ `(dyn Any + 'static)` cannot be sent between threads safely
   |
   = help: the trait `Send` is not implemented for `(dyn Any + 'static)`
   = note: required because of the requirements on the impl of `Send` for `std::ptr::Unique<(dyn Any + 'static)>`
   = note: required because it appears within the type `Box<(dyn Any + 'static)>`
   = note: required because it appears within the type `(TypeId, Box<(dyn Any + 'static)>)`
   = note: required because of the requirements on the impl of `Send` for `hashbrown::raw::RawTable<(TypeId, Box<(dyn Any + 'static)>)>`
   = note: required because it appears within the type `hashbrown::map::HashMap<TypeId, Box<(dyn Any + 'static)>, BuildHasherDefault<rustc_hash::FxHasher>>`
   = note: required because it appears within the type `HashMap<TypeId, Box<(dyn Any + 'static)>, BuildHasherDefault<rustc_hash::FxHasher>>`
   = note: required because it appears within the type `Option<HashMap<TypeId, Box<(dyn Any + 'static)>, BuildHasherDefault<rustc_hash::FxHasher>>>`
   = note: required because it appears within the type `type_map::TypeMap`
   = note: required because of the requirements on the impl of `Send` for `RefCell<type_map::TypeMap>`
   = note: required because it appears within the type `intl_memoizer::IntlLangMemoizer`
   = note: required because it appears within the type `FluentBundle<FluentResource, intl_memoizer::IntlLangMemoizer>`
   = note: required because of the requirements on the impl of `Sync` for `std::sync::Mutex<FluentBundle<FluentResource, intl_memoizer::IntlLangMemoizer>>`
   = note: 1 redundant requirement hidden
   = note: required because of the requirements on the impl of `Sync` for `Arc<std::sync::Mutex<FluentBundle<FluentResource, intl_memoizer::IntlLangMemoizer>>>`
```

Looking closely at the end of that error message it appears that the trouble comes from `intl_memoizer::IntlLangMemoizer` having a type? of `(dyn Any + 'static)`. I'm not quite familliar enough with Rust to understand how a type can have a type, but that's how I understand it...

A little snooping around the web with kagi tells me [I'm not alone](https://github.com/projectfluent/fluent-rs/issues/167). So I finally figured out how to do it. Strangely, I had to use fluent 0.14.4 because there is a regression in the latest version and the concurrent version of fluent bundle is missing. I looked through the git history and I could not figure out for the life of me how that happened. It just dissapears with no suspicious looking commits.

In the next session I am going to try to figure out how to include the fluent files as resources in the rust binaries and to load the locale from `LANG`.

{{<screencast "2023-03-22-292d553b-779d-4afd-82b1-f1b28e762c70" "99ce23b59c7f33b4b63e442443fd1f88">}}

{{<screencast "2023-03-23-f1cffe0c-e479-49cd-9396-9487de81d172" "99ce23b59c7f33b4b63e442443fd1f88">}}
