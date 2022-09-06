---
title: "Lets start putting together a road map for gradesta"
date: 2022-08-31
featureImage: https://assets.gradesta.com/gradesta/img/dalle2-clock-and-coins.png
author: Timothy Hobbs <tim@gradesta.com>
draft: true
---

Part 1: Just figuring out how to organize the tasks
--------------------------------------------------------------

In this part I think about how to organize the tasks and I start listing tasks for the manager.

{{<screencast "2022-8-31-42f5eafe-d710-41da-afeb-18125ee83c73">}}

Part 2: Developing a mechanism for gathering the tasks and summing their estimated time costs
--------------------------------------------

{{<screencast "2022-8-31-38358167-2830-4355-9a2c-e04479090446">}}

Part 3: Browser roadmap
----------------------------

{{<screencast "2022-8-31-e5ccc82a-5e53-4f3f-a7e1-fb8c0ca6b0b3">}}

Part 4: Milestones
----------------------

This is a coding session in which I write a script for extracting time estimate summaries per milestone.

{{<screencast "2022-9-1-8472d8b0-8c06-4d9f-8d05-0009400cffd8">}}

In the last part I will be looking at how to record time spent on each task by associating screencasts with tasks and recording a list of the screencast lengths and authorship.

Part 4: TASK: Setting up pytest for kcf task gathering code
---------

```
*** TASK_ID: cc0874cc357bfb9d81a91267dbb6cbbf
*** CREATED: 2022-09-01 19:05
*** ESTIMATED_TIME: W3
*** MILESTONES: kcf-task-management
```

{{<screencast "2022-9-1-f62403e9-09b6-4457-89bf-95d5a14b8e6c" "cc0874cc357bfb9d81a91267dbb6cbbf">}}

Part 5: TASK: Screencast metadata and config
------------

```
*** TASK_ID: bb20cd7d5ae49813c4ab36b7c33d4272
*** CREATED: 2022-09-03 20:46
*** MILESTONES: kcf-task-management
```

{{<screencast "2022-9-2-e8ff07d8-5148-4059-ab24-66b77988c577">}}

Part 6: TASK: Add tests for screencast config and metadata code
--------------

```
TASK_ID: 2748a899cdbd3cc1c99194386d328540
CREATED: 2022-09-04 05:51
MILESTONES: kcf-task-management
```

{{<screencast "2022-09-03-e6ad8aa9-4f96-493c-a7c1-4279f609876e">}}


Part 7: Add function for parsing hugo frontmatter
--------

In order to save screencast metadata, I need to parse frontmatter so I can extract the authors. This turned out to be extremely difficult and frustrating for two reasons:

1. I struggled to find a good way to parse a date with serde.

2. I struggled to get nom to give me good error messages. It turns out that nom isn't designed to give good error messages. I will have to switch to something better in the future.

{{<screencast "2022-09-04-e3e86b1a-7264-4409-bae9-71906fab2307">}}

Part 8: TASK: Recording screencast length on user upload
----

```
TASK_ID: 5a63bdcd7ee86ae33c0eba0525faf735
CREATED: 2022-09-01 19:03
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-task-management
```

{{<screencast "2022-09-04-59266aa1-467a-4f05-a4b8-593f5fb0704b">}}

Part 9: TASK: Associating screencasts with tasks
--------------------------------

```
TASK_ID: b4f4710742e5aa3fe4c920cd0192c04d
CREATED: 2022-09-01 19:01
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-task-management
```

{{<screencast "2022-09-06-4f595839-95a0-435c-ba70-b4862c235749" "b4f4710742e5aa3fe4c920cd0192c04d">}}

Part 10: Fixing kcf-tasks after moving it's dir and setting up argparse
-------

I actually start out discovering that kcf-tasks is no longer in my path. It turns out that this is due to the fact that in the previous part I moved the task-manager source directory to the tools directory. Apparently virualenv is not set up to handle such changes and so I had to delete the virtualenv and start over.

Next I fight with argparse and allowing the user to specify a path from which to gather the tasks. This leads me down all sorts of rabit holes involving my lack of ability to work with python as a dynamically typed language (it doesn't tell me that I'm an idiot when I do unreasonable things like try to pass a string to a function that expects a list...)

{{<screencast "2022-09-06-54ca2115-0145-4812-bd77-c3ed72d1cae9">}}

Part 11: TASK: Marking tasks as done and sumarizing completed tasks
-----


```
TASK_ID: aaff42e535b800c28d10d883177b5165
CREATED: 2022-09-01 19:04
ESTIMATED_TIME: W2
MILESTONES: kcf-task-management
```

