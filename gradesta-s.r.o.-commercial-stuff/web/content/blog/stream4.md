---
title: "Launching and configuring the manager"
date: 2022-08-06
featureImage: https://assets.gradesta.com/vegan-buddies/img/avomik.jpg
author: Timothy Hobbs <tim@gradesta.com>
draft: true
---

Part 1: Getting things building again after setting up mold (by dissabling mold)
------


We start out the session with the confusing error message

```
    Finished test [unoptimized + debuginfo] target(s) in 0.03s
     Running unittests (target/debug/deps/gm-12c52c9312f49a3a)
/home/timothy/pu/gradesta/manager/target/debug/deps/gm-12c52c9312f49a3a: error while loading shared libraries: libzmq.so.5: cannot open shared object file: No such file or directory
error: test failed, to rerun pass '--bin gm'
```

When running `cargo test`. It seems from [this article](https://matklad.github.io/2022/03/14/rpath-or-why-lld-doesnt-work-on-nixos.html) that this error may be being caused by [eariler work](https://veganbuddies.org/blog/stream17/) I did to make rust link faster. 

In the end I ended up just dissabling mold which solved the problem. If my new Ryzen processor also has slow linking problems I'll figure out how to fix it.

{{<screencast "2022-08-06-8d7e60bc62b16cf07726c6b0439c6490">}}

Part 2:
---------



{{screencast "2022-08-06-efc2c67b6d514a56e80c03fa956fb08d"}}
