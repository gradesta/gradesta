Public tender #3: Looking for where to host my videos
----------------------------------------------------------------

Keywords: video, hosting, blog
Max price: € 0,70 per hour to record and publish.

This list is from 1.2022.

I want to store and share/publish recordings of my live coding sessions so that the general public can see my progress and so I can document my work.

Cost to record at home:

10TB hard drive costs €370: 33 333 hours of recording. 16 666 hours with 2x duplication: €0.022 per hour.

Storage on [blackblaze.com](https://www.backblaze.com/b2/cloud-storage.html) costs $0.005 per GB per month. That is $0.20 per hour for 10 years of storage. + $3 per 1000 viewing hours.

1. [vimeo.com](https://vimeo.com/upgrade): 1TB per year for $35 per month. $0,14 per hour. No livestreaming. Automatic subtitles [in the future](https://vimeo.zendesk.com/hc/en-us/articles/224968828-Captions-and-subtitles#h_01FPAZBD10B1RVKG1QFWSTF85V)?  Embedding not GDPR compliant.

2. [spacebear.ee](https://federation.spacebear.ee/software/peertube) Peertube hosting: 50GB total storage $40 per month. $32 per hour when stored for 10 years. Not clear on subtitles, no automatic subtitles. Embeddint GDPR compliant.

3. [twitch.tv](https://help.twitch.tv/s/article/video-on-demand#enabling) Totally unclear on limits and pricing. Free maybe? Maybe deletes after 14/60 days? No subtitles.

4. [youtube.com](youtube.com) Free with Ads. Automatic subtitles. Embedding not GDPR complient

5. [IBM Video](https://www.ibm.com/products/video-streaming/pricing) $99 per month for 1TB storage. $3,96 per hour over 10 years. Automatic subtitles.

6. [Dacast](https://www.dacast.com/live-streaming-pricing-plans/) $468 per year 50GB of storage: $31,2 per hour over 10 years.

7. Blackblaze is not GDPR compliant

8. Digital ocean spaces + CDN: $0,02 per GB per month with $5 minimum. $0,80 per hour over 10 years

Conclusion: Either DO Spaces + youtube

Technical implementation details:

Store metadata locally, use [youtube-upload](https://github.com/tokland/youtube-upload) to upload.

For uploading to vimeo, use [vimeo API](https://github.com/vimeo/vimeo.py#uploading-videos).

Uploading to DO spaces using [a third party tool](https://github.com/s3tools/s3cmd)

Note, that DO spaces cannot host a static website so I'll use a "normal" static webhost and deploy using FTP to deploy the blog with the video streams.
