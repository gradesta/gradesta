Gradesta slide deck
---

I created the company Gradesta s.r.o. to finance the development of the gradesta protocol.

The gradesta protocol is a novel protocol for serving 'stitchable spreadsheets'. Stitchable spreadsheets share a lot with the 'zigzag' concepts developed by Nelson et. al. In the 1990s. They are spreadsheets, but you can create arbitrary links between cells. In mathematical terms, stitchable spreadsheets are directed graphs with at most four labeled(numbered) edges coming from each vertex (and an arbitrary number of incoming connections so "backlinking isn't always symmetric (but usually is)").

One of the key elements of the gradesta protocol is the ability to serve graphs with dynamic topologies and dynamic content. This, combined with a generic 'graph browser' allows us to create interactive user interfaces literally just by creating and modifying a data structure in our language of choice.

History
-------------------

I started working on gradesta back in 2016. At that time, Nelson had a patent on the zig zag datastructure. His patent expired in 2019.

One of the main reasons I started developing gradesta was that I had been researching assistive technology. I discovered that screen readers are extremely difficult to use. Screen readers are the software seriously visually impaired users use to interact with their computers, either via text to speech or braille. Today, a blind individual must memorize dozens of hotkey combinations just to read and answer email. I was driven by a question: can't we develop a user interface for blind people that would require nothing more than the arrow keys to navigate?

Simultaneously I had been obsessed with graphs and their relation to visual programming since at least 2012, when I developed a DAG based dataflow language called haskarrow. Indeed, I would trace my obsession back far further, to when I was 13-14 and would trace lines between buttons and event handlers on paper, and wondered why source code had to do so much to obscure the signal network that asynchronous gui programming represents.

I had these questions in mind when I found an opensource piece of software called '[fenfire](https://hackage.haskell.org/package/fenfire)' on the internet. Fenfire was created as a Finish spin off of Nelson's zigzag, designed to avoid patent infringement. It was written by Benja Fallenstein and Tuukka Hastrup in an outdated dialect of haskell and took nearly a week to revive. I immediately felt that it was what I was looking for. A simple way to navigate a graph using the arrow keys. I was convinced that this was the way to solve both problems at once.

Fast forward a few years, I had developed a piece of software called [textgraph](https://hobbs.cz/tg/tg.html). It was more than just a prototype, it was a fully functioning IDE with its own graph based dialect of python. The entire project was self hosting, including the compiler. But the textgraph protocol, which was served over unix pipes, didn't work very well. I needed something much faster and more stable.

I then developed [a new protocol](https://hobbs.cz/tg/tg/dev-docs/docs.html) in protobuf2 and developed a server in go, with clients in python. During the process I invented a topological query language for gradesta graphs. I call this new type of query a walktree. This query language is unlike semantic query languages like [cypher](https://en.wikipedia.org/wiki/Cypher_(query_language)), in that it doesn't look at the data that the graph holds, only the structure of the graph. The ability to quickly get a subgraph with a certain topology from a graph his is very important for serving (and displaying on the client) a topologically dynamic graph over the network. If requests have to fly back and forth for every vertex, the protocol would be very slow, but with walktrees the client can send a single request with a walktree and receive back all the vertexes that fit into the topology that its user interface is designed to display.

The concept of a walktree is fundamentally rather simple. It is a tree of walks where there is a root node and branches and each path between a leaf and the root represents a walk (along a graph). Vertexes in the trees are labeled according to directions (the edge labels in a gradesta graph).

Creating a full walktree and sending it across the wire would be quite inefficient. Luckily, walktrees are quite easy to compress, as they can be generated using context free grammars. With the addition of some counters and a few extra rules, the process can be made both efficient and powerful. "Advanced" walktrees can not only walk the graph but also answer questions about the graphs topology. As an aside, since walktrees are trees of walks, querying the graph can hypothetically be done in a parallel multithreaded manner.

This protobuf based version had its problems as well, I got too caught up in developing a complete and fully featured protocol, complexity got out of hand. I had the kitchen sink in there, encryption, encodings, role management... It was too much... Also, the server code was written in go and suffered from golang's lack of generics. This became especially tedious in the test suit where it was necessary to write a lot of repetitive code. Finally, there were serious problems with the portability of protobuf2. There are of course proto2 language bindings for just about every language imaginable, unfortunately, the behavior of optional fields isn't consistent between these bindings, I used optional fields heavily in the specification and realized that the risk of bugs was too great. I want gradesta to be served from every programming language so client libraries will have to be written for dozens of platforms. I can't go around planting semantic landmines in the form of inconsistent optional field behavior for the implementer to figure out.

I decided to start again, with [a third version](https://github.com/gradesta/gradesta). This new version of the protocol is much simpler and uses capnproto for encoding. Capnproto doesn't have optional fields so I can't shoot myself in the foot with them.

In the new protocol there are no walk trees, nor roles. There is optional client side encryption, though it is incapable of obscuring topologies. The protocol has essentially two functions to get and set vertex labels, and watch for changes in graph connectivity. This allows me to serve a graph though, and walk trees and RBAC and all that other stuff can be interacted with via a 'META' section of the graph itself. After all, serving walktrees in graph form kind of makes sense 😉.

Business model
=======

I was indoctrinated with Free software and leftist ideology from a young age, however I've found myself more and more frustrated by the apparent lack of spontaneous self organization of free people (doing exactly what I personally want, of course). Boring people seem to do things like drink beer in their free time and interesting people often seem to have their own ideas. If I want people to work on MY ideas, I have to find a way inspire them or to pay them. After mulling it over in my head, I decided for the later, mostly because inspiration alone doesn't put food on the table.

*Goodnight gradesta*

The first gradesta application I plan to make is called *goodnight gradesta*. It is essentially an audio based note taking app that allows you to record little snippets of audio and arrange them in a structured fashion. This is good for three things

- *editability* Unlike a recording of hours of raw audio, a collection of snippets can be edited, rearranged, snippets can be deleted and recorded anew to make changes.
- *skimmability* it is possible to skim visually presented text, reading for example on the titles, or first sentence of each paragraph. Using a structured set of audio snippets, we can skim the audio in the same way.
- *pacing* podcasts can be paused or played at 2x speed, but the user doesn't feel in control of the speed the way they do when they can use the arrow keys to move between sentences. This is especially useful for technical content.

Goodnight gradesta will be fully multimedia as well, allowing you to insert images into your audio essay. This allows for one very important feature, the ability to created an annotated photo album with voice notes, using one hand and a cell phone, to both take the photos and add the notes.

You can find a non-graphy non-mulitimedia prototype of goodnight [here](https://hobbs.cz/push-to-talk/index.html).

While most of the gradesta code will be open source, Goodnight will be published using a more restrictive, source available license that has yet to be created. This is both for business reasons and philosophical ones. I personally believe that the GPL was originally intended to remove the asymmetric power balance of the EULA which is a kind of one-sided contract that you have no choice but accept (or not use the software at all). I believe that the modern day equivalent of the EULA is the TOS. Lots of GPL and AGPL software is made available in services with a draconian TOS. Indeed, to me, the TOS is worse than the EULA, because unlike the EULA, the TOS can change at any time. My license would limit the scope of TOS that could be made and give end users GDPR style rights, like the right to back up and export all data, demand data be deleted ect. Finally, it would be impossible to forbid automated interaction with the service. Now-adays, it is often impossible to create third party clients or bridge protocols due to TOS restrictions. This is very good from a business standpoint, because these walled gardens have a strong monopolistic network effect, but its not good for society. There's still a lot of thinking to do before I figure out how to make such a license work, so at first I'll probably release the code CC-NC.

Finally, icons and other visual assets that form the 'product brand' will be protected both by copyright and trademark, likely being released under a creativecommons noncomercial licence.

Goodnight will work both locally and as a SaaS. Pricing will be a three tier system:

1. Slum: advertising funded, limited ability to store private content. Calling it a "slum" is a business decision. I want to cast shade on other slums like Facebook which are %100 slummy and don't have a pro version.
2. Edu: for schools
3. Pro: for everyone else

There will be two types of payment:

1. Monthly
2. Pay per minute of audio: Each minute will be stored for 10 years (unless the user deletes it) and this continued storage will be included in the price.

Advertising
---------

Only the free "slum" version of the product will have advertising.

As a child I found newspaper ads to be more interesting than todays 'personalized' content. I believe it is easier to put a relevant ad next to content than to try to figure out what is relevant to any given user. It seems like good business sense to get rid of all the tracking and just serve 'dumb' ads. Even 'dumb' ads can create metrics on whether anyone clicked on them, so someone will pay.

Advertising will be priced using a novel pricing model. Rather than paying when users are interested, advertisers will pay if a user hates their ad. Each ad will have a "burn button" next to it. If a user truly hates an ad, they can burn it, knowing this will cost the advertiser money. Users who burn ads will have to play a captcha like game of entering a 'launch code' into a virtual explosive device. Once they do so, they will get to watch with satisfaction as their hated advertisement burns up. Of course this will mean it will be much cheaper for children's cancer charities to advertise on the platform than scam artists. That is the intent anyways...

Actual pricing will be based on an auction with a low minimum price. Advertisements will be displayed at the ends of columns and rows of public content and will be sorted so that the highest bidder is closest to the content. Once an ad is payed for, it stays up until someone burns it.

One question is, with high priced slots, what prevents competitors from burning each others advertisements, except for the fear of prosecution for fraud. I don't really have the answer yet... But then again, traditional advertising already has this problem with fake clicks, and I'm pretty sure that the solution is that google just pockets the fraudulent ad spend.


Gradesta studio
-----

The second gradesta application I intend to develop will be a sort of stitchable spreadsheet 21st century excel. It will allow the users to create spreadsheets, and also link them to dynamic data sources also served over the gradesta protocol. The key feature of gradesta studio will be the **command bar**.

Rather than having functionality and menus built in, gradesta studio will have a command bar which searches a stack overflow like database of questions. Rather than having a button to sum a column, the user will search for 'sum column' and will recieve an 'answer' in the form of a command. These commands will be user generated and run in isolated containers client side. This code will be contained in such a way that it can be run even if it is untrusted. The worst that could happen is that it deletes your data and you have to press Ctrl-z to undo the action. If you type a command into the command bar that hasn't been implemented yet, a new 'question' will be created in a database of unanswered questions. Two things can now happen:

 - Another user can create a new command for you
 - another user can connect directly with your instance of studio and do the task for you right in front of your eyes.

Paying users will be able to upvote unanswered questions. Part of the money they pay will be payed out to whoever uploaded the commands they use. Direct support calls can either connect you to your colleagues, or to a paid agency.

Pricing would be:

 - free, with no ability to upvote unanswered questions but the ability to ask and answer questions
 - monthly and per user with ability to upvote questions
 - super premium with live support.
 - Maybe there would be the ability to buy upvotes as well.

One of the business model design goals would be to have the commands be open source and yet have their authors get payed when paying users use those commands. What would prevent some asshole from stealing the commands and presenting them as their own with the hope of getting payed? I don't know yet...

**Elves** one final very important aspect of gradesta studio is Elves. Remember walk trees? They allow you to subscribe to a section of the graph. Well they can also be used for access control. Elves are third party services that can access parts of your graphs. They can do stuff in the background. For instance, if you have a live datasource showing new orders, and a second table of invoices, than you can give an invoicing Elf access to your orders list, and it'll create invoices for you, even as you sleep! Connecting up elves is a weird no-code way of very high level programming. You basically just, find an Elf using the command bar, select the columns it should work on and it'll go to work. Elves can be paid for their work, and users already paying monthly won't even have to re-enter their credit card numbers, it should all be very seamless. Or as seamless as a stitchable spreadsheet can be (pun intended)

New search
--------

The third gradesta application I want to build would be a new kind of search engine. This would be quite different from google. Search history would be a central part of the product, rather than being more or less ephemeral. You enter a search and you sort through the results and choose what you want to keep and can add commentary to your search processed. Eliminated results are eliminated for the rest of the session, meaning when you're trying different key words and can't find anything, you don't have to sort through the same old BS over and over again. This is very usefull for knowledge workers. I often spend many hours just searching the web, find great stuff, and bookmark it, but the bookmarks are a jumble, lacking context. I want to create a 'search story' that I can share with collegues and friends. After reading for hours about protobuf, jsonb, flatbuffers and capnoroto, I don't want my decision process to just disappear, I want it to be preserved. I don't want you to look at my choice of capnpproto and think 'oh, he chose that at random' I want you to understand the alternatives I came across and why I chose what I did. At the same time, I don't want to copy links into some kind of academic paper, I want that story to appear organically in the 'search document' that I create.

Finally, I when I'm searching for a comercial product, I want to be able to let suppliers know about it and for them to "help me" find what I'm looking for. At the same time, I don't want them to harass me with irrelevant junk. If they do give me a whole bunch of irrelevant nonsense, I want to punish them. Therefore, not unlike the previously described advertisements in goodnight, I would allow advertisers answer my 'comercial inquiry' type searches. Answering inquires should be free, unless the answers are irrelevant. I would use a bidding system where the 'results' would be sorted by 'price', but the advertisers would ONLY pay if I burned their offers.

The only question is how to prevent competitors from burning eachother? Well in this case there could be a sort of reputation system, where the advertiser could see the 'spend to burn' ratio of the user who posted the inquiry. They would then chose to bid for a slot if that ratio seemed acceptable.


Enterprise certification program
----------- 

Almost all of the software I've described can make money using an open-core if not completely open source business model. However, if there was still doubt about the profitability of gradesta studio, there is another way of making money off of it: performing security audits on networks of Elves (Internal to the organization or otherwise) and dynamic data sources. We could find good means of static analysis and sell big organizations certain guarantees about their particular configurations. We could certify those configurations that we had audited and state that the certification is only valid if all users of the system were trained. We could then get payed to train users about encryption and security. We could also make a deal with some insurance companies, that they would provide insurance only on certified gradesta installations/configurations. This would make pricing easy:

If

- **R** is an arbitrary coefficient representing the risk and complexity of the system
- **M** is the max insurance payout
- **C** is the cost of the audits and training
- **P** is the desired profit margin
- **B** is the boring work fee

The organization would pay `(R*M + C + B)*P` for certification and insurance. Then we'd then divide that up fairly with the insurance company. That might seem cloudy, but its actually much simpler and more transparent than most enterprise pricing by a lot.
