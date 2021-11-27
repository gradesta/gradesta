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

Code Kaban Flow
------------------

We use a kaban like flow for writing code.

Code flows from the `work_table` to the `aging_cellar` to the `finished` folder to the `archive` folder.

```
╭────────────╮   ╭──────────────╮   ╭──────────╮   ╭─────────╮
│ work_table │ → │ aging_cellar │ → │ finished │ → │ archive │
╰────────────╯   ╰──────────────╯   ╰──────────╯   ╰─────────╯
```

We believe that code can be finished, at which point it should not be changed except in extreme
circumstances such as security bugs.

New code is always added to the `work_table` for a given subproject.
Once that code has no changes in 5 days according to `git blame`
and that code has %100 test coverage
and that code is fully documented (every function, interface, and datastructure)
it can be moved to the `aging_cellar`.

Once code is 5 days old, fully tested and documented it can be moved to the `aging_cellar`.
If you want to make changes to code in the `aging_cellar` you must move it back to the `work_table` and restart the process.

Once code has been in the `aging_cellar` for 7 months according to `git blame` it can be moved to the `finished` folder.

Finished code should have the following comment added to the top:

```
/*
COMPLETE: Do not change except for security fixes
COMPLETION DATE: YYYY-MM-DD

Authors:
List of names taken from a careful examination of git blame, one per line.

Auditors:
List of names, one per line.
(You can add your name to this list if you've read this file and found no errors. Don't forget to gpg sign your commit.)
*/
```

If you need to make functional changes to code in the `finished` folder, you must COPY that code to the `work_table` and give the symbols a new name.
For example, if you wanted to add the ability to add a `max_items` argument to the `map` function.
Rather than modifying the `map` function,
you should COPY the `map` function with a new name `map2` and give `map2` the extra argument.
If the old code in the `finished` folder is no longer used or needed, it may be moved to the `archive`.
Code that requires the old function, however, should not be updated to use the new function.
Is is worse to change an old, tested and audited code path than to duplicate code.

How PRs are processed
-------------------------

There are various types of PRs:

- `new-code`: Can change only `work_table` and `aging_cellar` folders and move code from `finished` to `archive`
- `audit`: Can only add names to the `Auditors` sections of code in the `finished` folder
- `flow`: May only move code from `work_table` to `aging_cellar` or from `aging_cellar` to `finished`
- `security`: May change code anywhere including `finished`
- `polish`: May only change code on `work_table` to add tests/documentation.
- `clean`: May only remove untested/undocumented code from `work_table`
- `roadmap`: Explained later

There are two phases of development: `open` and `freeze`.

During open development all types of PR are accepted.
If a maintainer declares a freeze, `new-code` PRs are no longer accepted until the freeze ends.

A freeze ends only once there is no longer ANY code in `work_table`.

Flow is important
--------------------

Rather than having PRs open for a long time, they should be merged or closed quickly.
Even if new code isn't perfect we should quickly put it on the work_table.
We can always clean it up later if no one pitches in to polish it.
If PRs don't follow the rules, they should be closed immediately with a polite explanation.
A new PR can always be opened.
PRs should always be tagged by type.

Issue flow is also important
----------------------------------

Issues should be either closed (if there is no way they will ever be fixed), or moved to the code repo.

Issues are stored in the code repo as markdown files.

Next to the `work_table`, `aging_cellar`, `finished` and `archive` directories, there is a `blackhole` directory.
Both the `blackhole` directory and the `work_table` directory contain issues.
Only maintainers can put issues onto the `work_table` but anyone can solve an issue in the `blackhole` directory with a `new-code` or `security` commit.

The `blackhole` directory has two subdirectories:

- `bug` which contains bugs. Bugs can be moved to the `work_table` directory by anyone by adding a failing test to the `work_table` directory.
- `feature`

The `feature` dir has two subdirs:

- `roadmap` contains feature requests that the maintainers want too.
- `wonderland` contains all other feature requests.

Feature requests can be moved to the `work_table` directory by adding an implementation to the `work_table` directory. They can also be moved to the `work_table` by a maintainer.

Pull requests that work with issues are tagged `roadmap`.
