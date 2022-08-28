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

Part 2: Refactoring config parsing code and linting various things in the manager
---------

{{screencast "2022-08-06-efc2c67b6d514a56e80c03fa956fb08d"}}

Part 3: Use a tokio main and print out non-garbled error messages and exit with the right exitcode
---------

{{screencast "2022-08-14-a2fa42c311bd0bdee43e2378d5672862"}}

Part 4: Polish and run lockfile code
--------

{{screencast "2022-08-14-6123c72fb3e6f15d517956926a0a6767"}}

Part 5: Cleaning up dangling sockets and orphaned services
---------

{{screencast "2022-08-14-1fa30af99995269ff537fde989bfcb19"}}

Part 6: More cleaning up of dangling sockets
---------

{{screencast "2022-08-15-8054f51c60c1ab3474b6a5218e34b7e3"}}

Part 7:  More cleaning up of dangling sockets
---------

{{<screencast "2022-8-21-11eddf31-9be5-4450-a7d7-8fdd02fda669">}}

Part 8: ofiles tests
---------

So I am struggling to understand what the proper type for PIDs are. `ofiles` used `u32` because that is what [the pid type](https://doc.rust-lang.org/std/process/fn.id.html) used by the rust standard library is. This is probably because that is [the pid type on Windows](https://docs.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getprocessid) (Microsoft defines a [DWORD](https://docs.microsoft.com/en-us/windows/win32/winprog/windows-data-types) as a `u32`), and in POSIX, negative Pids don't really exist so this is the safe type to use on both systems. However [`nix`](https://docs.rs/nix/0.25.0/nix/pty/type.SessionId.html) uses an `i32`, because that is the type used by [glibc](https://ftp.gnu.org/old-gnu/Manuals/glibc-2.2.3/html_node/libc_554.html) and [the linux kernel](https://github.com/torvalds/linux/blob/5147da902e0dd162c6254a61e4c57f21b60a9b1c/include/linux/pid.h#L55). Since `ofiles` is distinctly POSIX bound, yet there are never negative PIDs it seems best to use `i32` natively but implement `From` for `u32` as well.

{{<screencast "2022-8-22-2ac231f3-6fdd-4f83-8b19-b24b40c065dd">}}


Part 9:
----------

Still working on `ofiles` Pid type.

{{<screencast "2022-8-28-839391cb-4af4-41c5-9046-8e2367836cca">}}

Part 10:
---------

Implementing tests for clean_socket_dir using the [test_binary](https://docs.rs/test-binary/latest/test_binary/) package.

{{<screencast "2022-8-28-2ddff790-a50c-44dc-8490-4cc23c769360">}}

