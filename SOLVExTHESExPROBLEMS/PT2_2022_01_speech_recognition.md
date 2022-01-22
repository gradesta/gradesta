Public tender #2: Speech recognition
-------------------------------------------

Goodnight gradesta is an audio native note taking app. It allows you to record short snippets of audio and work with them as easily as you work with text. While audio is a first class citizen in goodnight, I still want to be able to add text previews to speech snippets, to make them easier to work with. For this, I need great speech recognition that works efficiently with very short snippets of speech. Ideally, it should support many languages.

I found [this list](https://fosspost.org/open-source-speech-recognition/) of open source speech recognition engines online. It is not clear to me how good they are, or whether they come with training data or need to be trained. If they need to be trained, then they are useless to me, I need something that works out of the box.

I also found [this list](https://blog.api.rakuten.net/top-10-best-speech-recognition-apis-google-speech-ibm-watson-speechapi-and-others/) of proprietary cloud based speech recognition APIs. Most charge either by the minute or 15 second intervals. One thing that is unclear to me, is if the by the minute charges are for full minutes or if fractional minutes are charged fractionally. If it is for full minutes then speech recogition is going to be extremely expensive, as I will have a large number of snippets that are only a few seconds long each.

If you have more information about speech recognition APIs, services, or software. Please create a PR.
