Kanban Code Flow
------------------

We use a kanban like flow for writing code.

Code flows from the `work_table` to the `ageing_cellar` to the `finished` folder to the `archive` folder.

```
╭────────────╮   ╭───────────────╮   ╭──────────╮   ╭─────────╮
│ work_table │ → │ ageing_cellar │ → │ finished │ → │ archive │
╰────────────╯   ╰───────────────╯   ╰──────────╯   ╰─────────╯
                                           │
                                           │     ╭────────────────╮
                                           ╰───▸ │ public_archive │
                                                 ╰────────────────╯
```

We believe that code can be finished, at which point it should not be changed except in extreme
circumstances such as security bugs.

New code is always added to the `work_table` for a given subproject.
Once that code has no changes in 5 days according to `git blame`
and that code has %100 test coverage
and that code is fully documented (every function, interface, and datastructure)
it can be moved to the `ageing_cellar`.
Code can only be moved on if all its imports are moved with it or are already there.
The process of isolating a unit of code into a single file with all tests and documentation and moving it to the `ageing_cellar` is referred to as "packing".
Finished and ageing code cannot depend on code on the `work_table`
and finished code cannot depend on code in the `ageing_cellar`.

Once code is 5 days old, fully tested and documented it can be moved to the `ageing_cellar`.
If you want to make changes to code in the `ageing_cellar` you must move it back to the `work_table` and restart the process.

Once code has been in the `ageing_cellar` for 7 months according to `git blame` it can be moved to the `finished` folder.

Finished code should have the following comment added to the top:

```
/*
COMPLETE: Do not change except for security fixes and critical errors

COMPLETION DATE: YYYY-MM-DD

RECALL LOG:
List of all changes by date (most recent comming first) of security and critical error fixes.

Authors:
List of names taken from a careful examination of git blame, one per line.

Auditors:
List of names, one per line.
(You can add your name to this list if you've read this file and found no errors. Don't forget to gpg sign your commit.)
*/
```

If you need to make functional changes to code in the `finished` folder, you must COPY that code to the `work_table` and give the symbols a new name and REMOVE the "Auditors" section.
For example, if you wanted to add the ability to add a `max_items` argument to the `map` function.
Rather than modifying the `map` function,
you should COPY the `map` function with a new name `map2` and give `map2` the extra argument.
Code that requires the old function, however, should not be updated to use the new function without good reason.
Is is worse to change an old, tested and audited code path than to duplicate code.

If the old code in the `finished` folder is no longer used or needed or publicly exported, it may be moved to the `archive`.
Finished code that IS publicly exported, but no longer recommended for use may be moved to the `public_archive`.
When code is moved to the `public_archive` a note should be added to its documentation, explaining that it is no longer recommended, why it is no longer recommended, and the recommended alternatives.

How PRs are processed
-------------------------

There are various types of PRs:

- `new-code`: Can change only `work_table` and `ageing_cellar` folders and move code from `finished` to `archive`
- `audit`: Can only add names to the `Auditors` sections of code in the `finished` folder
- `flow`: May only move code from `work_table` to `ageing_cellar` or from `ageing_cellar` to `finished`
- `recall-security` and `recall-error`: May change code anywhere including `finished`. Must list changes in `recall_log.md`
- `polish`: May only change code on `work_table` or in `ageing_cellar` to add tests/documentation.
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
We can always clean it up later (deleted) if no one pitches in to polish it.
Code is deleted from the `work_table` because no one polishes it,
it can always be submitted again with more tests and better docs.
If PRs don't follow the rules, they should be closed immediately with a polite explanation.
A new PR can always be opened.
PRs should always be tagged by type.

Issue flow is also important
----------------------------------

Issues should be either closed (if there is no way they will ever be fixed), or moved to the code repo.

Issues are stored in the code repo as markdown files.

Pull requests that work with issues are tagged `roadmap`.

Next to the `work_table`, `ageing_cellar`, `finished` and `archive` directories, there is a `blackhole` directory.
Both the `blackhole` directory and the `work_table` directory contain issues.
Only maintainers can put issues onto the `work_table` but anyone can solve an issue in the `blackhole` directory with a `new-code` or `recall-security` or `recall-error` commit.

The `blackhole` directory has two subdirectories:

- `bug` which contains bugs. Bugs can be moved to the `work_table` directory by anyone by adding a failing test to the `work_table` directory.
- `feature`

The `bug` dir has two subdirs:

- `reproduced` which contains bugs that have happened to multiple people and ideally have an explanation on how to reproduce them
- `unclear-unreproduced` which contains all other bugs

The `feature` dir has three subdirs:

- `roadmap` contains feature requests that the maintainers want too
- `rejected` contains rejected feature requests along with a polite and sufficient explanation as to why the request was rejected
- `wonderland` contains all other feature requests


Feature requests can be moved to the `work_table` directory by adding an implementation to the `work_table` directory. They can also be moved to the `work_table` by a maintainer.

So here is the "full flow":

```
╭───────────╮   ╭────────────╮   ╭───────────────╮   ╭──────────╮   ╭─────────╮
│ blackhole │ → │ work_table │ → │ ageing_cellar │ → │ finished │ → │ archive │
╰───────────╯   ╰────────────╯   ╰───────────────╯   ╰──────────╯   ╰─────────╯
                                                          │
                                                          │     ╭────────────────╮
                                                          ╰───▸ │ public_archive │
                                                                ╰────────────────╯

```



Code Conventions
-------------------

The Kanban Flow system described above works best with simple data structures and functions.
It is more or less incompatible with object oriented programming.
You don't want to keep a class on the `work_table` forever
just because you keep on adding methods to it.
Functions are much more self-sufficient and work much better with this system.

Recalls
--------

In case a finished piece of code is found to cause a security flaw or has a critical error it may be recalled. When code is recalled, the error is fixed and the recall is logged in `recall_log.md`. Each recall is given an Id in the format YYYY-MM-<one-word-descriptor>. A critical error is one that affects existing users and is not related to the support of a new platform or use-case. Besides critical errors and security flaws finished code should never be changed, only ever replaced. In some rare instances, for example when an encryption algorithm is found to be broken and is no longer capable of securely encrypting data, code may be recalled without a fix. In this case, the code file remains with a description of the recall but the code is removed.

List of top level project files and directories
---------------------------------------------------------

 - `work_table` - Directory containing actively worked on code
 - `aging_cellar` - Fully tested and documented code that has not yet been set in stone
 - `finished` - Fully tested and documented code who's functionality has stood the test of time and is set in stone
 - `archive` - Old finished code that is no longer used
 - `public_archive` - Old finished code that is publicly interfaced but who's use is not recommended
 - `blackhole` - Directory with issues
 - `recall_log.md`- list of security and critical error recalls against finished code
 
A Template for KCF (Kanban Code Flow) projects can be found in the `template` directory.
 
CONTRIBUTING.md Copyright and License
---------------------------------------------

This `README.md` document is copyright 2021 Timothy Hobbs <https://hobbs.cz>

You may use it in accordance to the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license
