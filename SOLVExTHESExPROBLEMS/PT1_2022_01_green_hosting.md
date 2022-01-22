Public tender #1: Shopping for servers
----------------------------------------------

Keywords: vps, hosting, dedicated server

This list is from 1.2022.

According to our ethical codex we should be protecting biodiversity, but typical web hosting can be rather nasty. Some if it is even powered by lignite coal which isn't typically good for biodiversity (though the nature reserves created from exhausted lignite mines are actually some of the most biodiverse places in Czechia).

Looking around I am faced with several hosting challenges:

1. Audio native computing is going to require a lot of bandwidth and storage space. Egress can be expensive or capped entirely. And VPSes don't often have much storage space. There are services like S3 are any of them eco?

2. If I'm going to charge reasonable prices and get stinkin rich, I shouldn't pay too much for it.

3. Hypothetically it's best in the EU or someplace with some semblance of privacy laws. Not sure if that's feasible. Not sure what GDPR regulations say on this. [The EU website](https://ec.europa.eu/info/law/law-topic/data-protection/reform/rules-business-and-organisations/obligations/what-rules-apply-if-my-organisation-transfers-data-outside-eu_en) says I should stick to countries that the European Commission has made an ‘Adequacy Decision’ for.

4. I need the service to work well, not have unexpected fees, and give me all the right access to the servers ect.

The options I've found:

1. [netcup.de](netcup.de)
 - Very reasonably priced VPSes: 2 vCore, 8GB ram, 80TB transfer for €10 a month (€6 when billed annually).
 - Claims to run on hydro power
 - Hosted in Germany
 - I tried to sign up and received a bizarre and not quite English email telling me to 
 
 ```
 send us a picture which shows you and your id card (it has to be readable in english or german).

To allocate your order to the sent proof please show additionally a
piece of paper with your order - ID: vz18414325
on this picture.

Additionally we would need a proof of address in English or German.
In case of doubt, an electricity bill in your name and address will also help.

The verification is performed to keep your personal data safe."
```
 - Has terrible reviews on [hostadvice.com](https://hostadvice.com/hosting-company/netcup-reviews/#main-info).
 
2. [kualo.com](kualo.com)
 - Very expensive VPSes: 2 vCores, 8Gb ram, 5TB transfer for €72 a month
 - Certified green power from the open grid (no guarantee that the actual power used is actually green)
 - Hosted in UK
 - Great reviews on [hostadvice.com](https://hostadvice.com/hosting-company/kualo-web-hosting-reviews/) but not very many of them.
 
3. [greengeeks.com](greengeeks.com)
 - Very expensive VPSes: 6 vCPU 8GB Ram, 10TB transfer for $110 USD a month
 - "300%" "green energy" with no explanation of what exactly that means
 - Hosting available in the Netherlands
 - I have hands on experience with their wordpress hosting, that it's maybe a bit hands on. But I don't think that has anything to do with VPSes
 - Tried the sign up flow and they immediately asked for a domain name. Really not sure how that's relevant to a VPS.
 - Typical reviews on [hostadvice.com](https://hostadvice.com/hosting-company/greengeeks-reviews/)
 
4. [ecohosting.ie](ecohosting.ie)
 - Somewhat more reasonably priced VPS 4 CPUs, 8GB ram 20TB transfer with ability to buy more for €1.23 a month
 - Website not very fancy
 - wind and hydropower in Finland, in Germany "certified green" power from the grid
 - Hosting in Finland and Germany
 - No reviews, no results on search
 - Seems to be a reseller, so how do I buy from those data centers directly?
 
5. [hydro66.com](hydro66.com)
 - VPS 4CPU 8GB RAM 5TB Transfer: €65 a month
 - Hydro powered
 - Location: Sweden
 - Couldn't find any reviews except for "articles" and blogspam
 
6. [datacenterlight.ch](datacenterlight.ch)
 - VPS 2CPU 8GB RAM (But just 10GB disk space) for €40 a month €80 for 120GB disk space [Transfer is not capped but is throttled](https://redmine.ungleich.ch/projects/open-infrastructure/wiki/FAQ_at_Data_Center_Light#Bandwidth)
 - Hydro + solar appears to be [as close to green as you can get](https://ungleich.ch/en-us/cms/blog/2019/06/28/how-run-really-green-datacenter/)
 - Switzerland
 - [Super open](https://datacenterlight.ch/en-us/cms/open-infrastructure-project/)
 - No reviews
 - Decided to try them out because I'm bored searching

Conclusion of search
------------------------

Right now the market is not doing a good job of providing affordable and ecologically powered servers. Or at least I cannot find them. All of the options I found have major drawbacks. Most have very low data transfer limits, datacenterlight has extremely high disk space costs. With the limited bandwidth it will be necessary, if I am to use green power for servers at all, to divide up compute and storage. Storing in CDNs and S3 compatible services that use traditional power sources and computing in the green places. Luckily, I planned around the ability to divide the storage of media files from the serving of gradesta cells.

Ideally, media files on the edge would be encrypted. One possible flow diagram would be.

```

  ╭──────────╮                       ╭──────────────────────────────╮
  │ client   │ ←-gradesta protocol-→ │ hydropowered gradesta server │
  ╰──────────╯                       ╰──────────────────────────────╯
        ↑
        │                                 ╭───────────────────────────╮
        ╰─ encrypted and signed media ──▸ │ coal powered media server │
                                          ╰───────────────────────────╯

```

However, this doesn't account for automatic transcription of audio using STT (Speech recognition).

The real setup will probably look something like [this architecture diagram](../gradesta-s.r.o.-commercial-stuff/top_secret/network_architecture.md).

Request for help
-------------------

If you have information about green hosting services that I've missed, don't hesitate to create a PR.
