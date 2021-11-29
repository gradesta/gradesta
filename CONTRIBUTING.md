Guide to Contributing to the Gradesta open source project
========================================================================

This is the monorepo for the gradesta open source project.
It includes the gradesta manager,
a gradesta websocket server
some example and useful gradesta services
library code for use by third parties when creating gradesta services / alternate websocket servers.
Third party code pulled in with `git submodule`

Library code is LGPLv3
Service, manager, and server code is AGPLv3.
Directories are marked with their licenses using the `LICENSE.md` file.
Unmarked files and directories should be assumed to be all rights reserved Gradesta s.r.o.

Before you start investing time in creating Pull Requests it is important to understand our development philosophy:

Move SLOW and do things ONCE
----------------------------------

This is almost exactly the opposite of the "Move fast and break things" philosophy that you're probably used to.

We avoid making breaking changes to our own libraries and external interfaces.
Lets not waste other people's time by renaming symbols and moving them around.
Similarly, if we find our dependencies break builds for us frequently,
more than once every 4-5 years excepting security fixes,
we either pin versions or move to an alternate, more stable library.
We'd rather use unmaintaned code than waste our lives renaming things and adding/removing function arguments.
We believe that security comes from good code and auditing and NOT from frequent changes.
Maintained != secure
Unmaintaned != insecure
Audited, tested, well written == secure

In this spirit, you should note that PRs that change level 0 of the gradesta protocol will almost certainly be rejected.
Changes that rename external interfaces/protocols or change them in a way that could break third party code will almost certainly be rejected.
We believe that it is better to create a new protocol every 5-6 years,
with a new name that doesn't conflict with the old protocol,
and which in no way interferes with the functioning of applications written for the old protocol,
then it is to constantly refactor an existing protocol.

Kaban Code Flow
------------------

Rather than a more traditional software development approach we use the [Kaban Code Flow model](./kcf/README.md)
