---
title: "Better screencast management"
date: 2022-07-08
featureImage: https://assets.gradesta.com/vegan-buddies/img/avomik.jpg
author: Timothy Hobbs <tim@gradesta.com>
draft: true
---

{{<screencast "2022-07-08-3f1fe08707a4ce6932e5282b8cf30ff9">}}
(Part 1)

In this screencast I am setting up a cleaner and less fiddly system for uploading videos of screencasts and pairing them with blogposts. Instead of generating the names for screencast files after the fact when the screencasts are uploaded. We first generate the file names and then set the names of the uploaded video assets according to the generated names.

To generate a new screencast name, you run:

```
./gradesta-s.r.o.-commercial-stuff/interal-tooling/mk_screencast_label
```

To publish a screencast, you then run

```
./gradesta-s.r.o.-commercial-stuff/interal-tooling/publish_screencasts <name-of-blogpost> <name-of-screencast-file.mkv>
```

You can have multiple screencasts per blogpost, you just list them as multiple arguments to the `publish_screencasts` commandand.

```
./gradesta-s.r.o.-commercial-stuff/interal-tooling/publish_screencasts <name-of-blogpost> <name-of-screencast-file.mkv>  <name-of-screencast-file-2.mkv>
```


The `publish_screencasts` command uploads the screencasts to an EC2 spot instance with transcodes them from OBS's native `mkv` format to the web's much smaller and smoother streaming `webm` format. The ec2 spot instan the screen cast to DO spaces.

![diagram showing flow of screecasts during publishing](/images/blog/publish-screecasts-flow.png)

{{<screencast "2022-07-09-669cde795a73e3aec1fdc1fd82929c9b">}}
(Part 2)

So I've decided not to transcode the videos on the cloud afterall. I don't have the time to set it up and its not strictly necessary. Instead, I will simply transcode locally using ffmpeg as I have done in the past. I still need to write a script that extracts the screencast id tags from the blog entries, does the transcoding locally, and uploads them to the CDN.

Part 3:
--------

{{<screencast "2022-07-24-657a1bb245023357b9a37519f01b4fea">}}

Part 4:
---------

{{<screencast "2022-07-24-0b0a47829b3a9f8f57f19b7dd7c01117">}}

Part 5: parsing out tags with nom
---------------------------------------

{{<screencast "2022-07-25-26d4043b2ab967ea24db5d20dd85c99e">}}

Part 6: Building the upload command dependency DAG and running the upload commands
--------------------------------------------------------------------------------------

I use the [daggy](https://docs.rs/daggy/latest/daggy/) package to build a dependeny DAG of commands that are run in parallel, in order of dependeny fulfillment, sort of similar to `make`.

{{<screencast "2022-08-03-a79910b214ad854b387814f1be3a57fe">}}


