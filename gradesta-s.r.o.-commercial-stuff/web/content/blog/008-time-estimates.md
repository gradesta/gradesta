---
title: "Analyzing the quality of time estimates and actual time spent"
date: 2022-10-25
featureimage: https://assets.gradesta.com/gradesta/img/dalle2-clock-and-coins.png
author: timothy hobbs <tim@gradesta.com>
draft: false
---

Part 1: Loading time spent metadata
-----------

Gathering the data from the screencast metadata and loading it into the kcf-tasks tool.

```
TASK: Pairing screencast durations with tasks in kcf_tasks code
TASK_ID: daad2f126cd66db5339249734364aae8
CREATED: 2022-10-23 20:42
ESTIMATED_TIME: W2 DONE
MILESTONES: kcf-tasks/time-spent
```

{{<screencast "2022-10-23-73307410-e611-4f5f-bf66-e9b14133f89a" "daad2f126cd66db5339249734364aae8">}}

Part 2: Continuing with loading tasks from metadata
--------

{{<screencast "2022-10-25-b3c92483-e32f-4c38-9f83-2fbf53282601" "daad2f126cd66db5339249734364aae8">}}

Part 3: Pairing time investments with timestamps/dates
------------------------------------------------------------------

I'm interested in how much individual "keyboard time" I spend vs how much time I thought I would spend. I also am kind of interested in how the future estimate changes as I add new tasks. If the finish line is getting closer, or farther away...

```
TASK: Pairing screencast dates with tasks
TASK_ID: 0edbdaaa9e469846ff237523f03640ee
CREATED: 2022-10-26 10:49
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-tasks/time-spent
```

{{<screencast "2022-10-26-11a8a013-5c46-4930-b2ce-4518ac45acec" "0edbdaaa9e469846ff237523f03640ee">}}

It turns out that I need to load the screencast dates and pair those with the tasks in such a way that I can graph the actual time spent. So that has become the subtask which I'll actually end up doing in this part.  We can find the screencast date by looking at it's ID, which is in the filename of the screencast metadata file.

Part 4: Add author data to work logs
-------------------------------------------

I thought this was going to be a very simple task, but it turns out that authorship data has not been making its way into screencast metadata. Will have to examine that further...


```
TASK: Pairing authorship data with time logs
TASK_ID: 47e0901b6b2248ba7c38420ecc7b1043
CREATED: 2022-11-04 17:39
ESTIMATED_TIME: W2 DONE
MILESTONES: kcf-tasks/time-spent
```

{{<screencast "2022-11-04-1c534448-8be0-442e-9adc-3d5afdc63c9c" "47e0901b6b2248ba7c38420ecc7b1043">}}

Part 5: Miscellania
----------------------

{{<screencast "2022-11-05-80bfdf9a-c15d-4977-af1a-8bedbdf19ab9" "7938687258bb97ab6699a011ee7cdd6c">}}

Part 6: Fix CI/CD errors in kcf code
-------------------------------------------

Since I started actually using some dependencies, I had to set up the github action to install those dependencies in a virtualenv in order to have the test"s" pass in the ci.

```
TASK: Fix CI/CD errors in kcf code
TASK_ID: 27bf7514314933ff56da2b58c5aa0da5
CREATED: 2022-11-05 10:22
ESTIMATED_TIME: W3
MILESTONES: kcf-tasks/time-spent
```

{{<screencast "2022-11-05-be1bf5dc-74c0-4bb7-a69d-944a3d7d8bda" "27bf7514314933ff56da2b58c5aa0da5">}}

Part 7: Comparing task estimates vs actual time spend
------------------------------------------------------------------------

I initially listed task 28c9fe4ca2a1a2ea604eea0e6aee3ca0 "Comparing time estimates with actual time spent" but realized that it needs to be broken into subtasks. The first subtask I ended up starting was 

```
TASK: Listing milestones by estimated time spend vs actual time spend
TASK_ID: 31a55ad0bb8f87121a23ff95c81fe558
CREATED: 2022-11-05 11:58
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-tasks/time-spent
```

however, half way through it occured to me that I already showed actual time spend next to milestones. I didn't realize that I needed to also show previously estimated time spend for already completed tasks to make a reasonbable comparison.

After that, I did

```
TASK: Listing completed tasks with actual vs estimated time spend
TASK_ID: ce205fc194d577549e245aaa8a09d6cc
CREATED: 2022-11-05 11:57
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-tasks/time-spent

TASK: Comparing time estimates with actual time spent
TASK_ID: 28c9fe4ca2a1a2ea604eea0e6aee3ca0
CREATED: 2022-09-01 19:04
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-tasks/time-spent
```

Here is how the task listing looks for completed tasks:

```
DONE in 0:35:34.700000 estimated 0:15:00-1:00:00: Set up CI to do `cargo fmt`
DONE in 0:22:36.200000 estimated 0:15:00-1:00:00: Set up precommit hook to do `cargo fmt` everywhere
DONE in 0:27:05.933000 estimated 1:00:00-16:00:00: Figure out why the test `ageing_cellar::organize_sockets_dir::tests::test_socket_dir_old_socket` is flaky
DONE in 1:57:08.333000 estimated 1:00:00-16:00:00: Set up CI to show code coverage
DONE in 1:57:08.333000 estimated 0:15:00-1:00:00: Set up CI to test kcf code
```

after that, I returned to the task involving milestone assessment and completed that task as well.

Currently the output for the milestones is as follows:

```
MILESTONE:  without-milestone
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                7

MILESTONE:  all-tasks
Minimum decision time:           150 days, 6:30:00
Maximum decision time:           538 days, 8:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    163.75  hours
Maximum individual work time:    1469.0  hours
Original estimate for complete:  8.0  →  80.0  hours
Total time spent:                17:01:25.163000
Completed tasks:                 20
Incomplete tasks:                90

MILESTONE:  mvp
Minimum decision time:           10 days, 6:15:00
Maximum decision time:           55 days, 4:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    61.25  hours
Maximum individual work time:    593.0  hours
Original estimate for complete:  4.75  →  57.0  hours
Total time spent:                14:21:39.997000
Completed tasks:                 10
Incomplete tasks:                46

MILESTONE:  ci
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.5  hours
Maximum individual work time:    4.0  hours
Original estimate for complete:  4.75  →  57.0  hours
Total time spent:                14:21:39.997000
Completed tasks:                 10
Incomplete tasks:                2

MILESTONE:  unix-sockets
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    1.0  hours
Maximum individual work time:    16.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2

MILESTONE:  manager-mvp
Minimum decision time:           7 days, 2:00:00
Maximum decision time:           37 days, 0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    27.25  hours
Maximum individual work time:    241.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                21

MILESTONE:  cursor-sharing
Minimum decision time:           3 days, 0:00:00
Maximum decision time:           14 days, 0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    9.0  hours
Maximum individual work time:    80.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                4

MILESTONE:  websockets
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    8.0  hours
Maximum individual work time:    64.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2

MILESTONE:  auth
Minimum decision time:           115 days, 0:00:00
Maximum decision time:           350 days, 0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                4

MILESTONE:  encryption
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    9.5  hours
Maximum individual work time:    84.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                4

MILESTONE:  webscale
Minimum decision time:           1 day, 0:00:00
Maximum decision time:           7 days, 0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2

MILESTONE:  javascript-mvp
Minimum decision time:           3 days, 0:00:00
Maximum decision time:           14 days, 0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    16.5  hours
Maximum individual work time:    172.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                16

MILESTONE:  browser-mvp
Minimum decision time:           0:15:00
Maximum decision time:           4:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    15.0  hours
Maximum individual work time:    144.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                8

MILESTONE:  custom-elements
Minimum decision time:           7 days, 0:00:00
Maximum decision time:           28 days, 0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2

MILESTONE:  audio
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    12.0  hours
Maximum individual work time:    96.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                3

MILESTONE:  images
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    8.0  hours
Maximum individual work time:    64.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                3

MILESTONE:  video
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    8.0  hours
Maximum individual work time:    64.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                3

MILESTONE:  file-attachments
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    9.0  hours
Maximum individual work time:    80.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                3

MILESTONE:  read-write
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    34.0  hours
Maximum individual work time:    288.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                9

MILESTONE:  keyboard-accessibility
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    1.0  hours
Maximum individual work time:    16.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2

MILESTONE:  critters
Minimum decision time:           14 days, 0:00:00
Maximum decision time:           84 days, 0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2

MILESTONE:  kcf-tasks/overhead-tasks
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.5  hours
Maximum individual work time:    4.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2

MILESTONE:  kcf-tasks/time-spent
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    2.5  hours
Maximum individual work time:    20.0  hours
Original estimate for complete:  1.5  →  10.0  hours
Total time spent:                0:00:00
Completed tasks:                 4
Incomplete tasks:                6

MILESTONE:  kcf-task-management
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  1.75  →  13.0  hours
Total time spent:                2:39:45.166000
Completed tasks:                 6
Incomplete tasks:                1

MILESTONE:  fast-ci
Minimum decision time:           0:15:00
Maximum decision time:           4:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                2
```

And if we zoom in on something complete like:

```
MILESTONE:  ci
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.5  hours
Maximum individual work time:    4.0  hours
Original estimate for complete:  4.75  →  57.0  hours
Total time spent:                14:21:39.997000
Completed tasks:                 10
Incomplete tasks:                2
```

We can see that I did the tasks in 14.5 hours and estimtated that they would take 4.75 to 57 hours. So I completed (most of) that milestone on schedule in agregate. However, we still don't know how the estimate evolved over time. Perhaps most of those tasks were added at the last minute or during development, which would make the estimate far less valuable...


{{<screencast "2022-11-05-959d265f-7ae6-4c3f-bfa4-1802ab819576" "ce205fc194d577549e245aaa8a09d6cc 31a55ad0bb8f87121a23ff95c81fe558 28c9fe4ca2a1a2ea604eea0e6aee3ca0">}}

Part 8: Creating a table to compair time estimate types vs actual time spend
---------------------------------------------------------

This will let me find out how long each type actually takes so that I can see if I'm any good at estimates or not.

```
TASK: Listing estimate type by actual time spend
TASK_ID: 5096c557109aa1292c40b00b39d76518
CREATED: 2022-11-05 11:58
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-tasks/time-spent

So it would print a table like:

   estimate             actual average   n accuracy low  high
W1 5min - 45 min        37 min           3 100%       0%   0%
W2 15min - 1 hour       45 min           7 73%        3%   7%
......
```

After having done this task, my current statistics are:

```
       estimate         actual average    n accuracy low  high
W2 0:15:00 - 1:00:00           0:54:58    8 25%      38%   38%
W3 0:30:00 - 4:00:00           0:22:49    7 29%      72%    0%
W4 1:00:00 - 16:00:00          2:10:33    3 67%      34%    0%
```

{{<screencast "2022-11-05-69228f99-c194-4dc6-b0b7-0ae60012b9ce" "5096c557109aa1292c40b00b39d76518 28c9fe4ca2a1a2ea604eea0e6aee3ca0">}}

Part 9: (preparing for) Graphing task creation and deletion over time
----------------------------------------------------------------

So I finally wanted to get started on

```
TASK: Graphing estimated vs actual time investment
TASK_ID: 5ab6edc3f28466cb6bbfbb811bba78d3
CREATED: 2022-10-23 20:43
ESTIMATED_TIME: W3
MILESTONES: kcf-tasks/time-spent
```

just kidding :D but I realized that I should really be breaking it up into sub-parts. It's deffinitely not going to be a `W3` more like a `W4` I think. So lets get to it. My next task is to:

```
TASK: Filtering task list output by milestone
TASK_ID: 46afe22b50d294cd8374fdc7293fc46c
CREATED: 2022-11-05 16:01
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-tasks/time-spent
```

I think that it would make a lot of sense to have some kind of hierarchical "directory structure" for milestones. Like rather than having this milestone be `kcf-task-management-time-spent` we would write `kcf-tasks/time-spent`. Then when we ran `kcf-tasks --milestone=kcf-tasks/ list-tasks` then we would get all tasks related to `kcf-task` milestones. So that's what I'm going to do.

Now the output can be like this:

```
[nix-shell:~/pu/gradesta]$ kcf-tasks --milestone ci
Could not decode file  /home/timothy/pu/gradesta/docs/gradesta-level-0.pdf
Could not decode file  /home/timothy/pu/gradesta/frontend/wireframes/gradesta-mobile.fig
Could not decode file  /home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/static/images/blog/publish-screencasts-flow.png
Could not decode file  /home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/static/images/favicon.png
Could not decode file  /home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/static/images/hero/hero-mask-svg.png
Could not decode file  /home/timothy/pu/gradesta/licenses/archive/AML/AML.xoj
Could not decode file  /home/timothy/pu/gradesta/protocol/gradesta-level-0.odt

MILESTONE:  without-milestone
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.0  hours
Maximum individual work time:    0.0  hours
Original estimate for complete:  0.0  →  0.0  hours
Total time spent:                0:00:00
Completed tasks:                 0
Incomplete tasks:                1

MILESTONE:  all-tasks
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.5  hours
Maximum individual work time:    4.0  hours
Original estimate for complete:  4.75  →  57.0  hours
Total time spent:                14:21:39.997000
Completed tasks:                 10
Incomplete tasks:                2

MILESTONE:  mvp
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.5  hours
Maximum individual work time:    4.0  hours
Original estimate for complete:  4.75  →  57.0  hours
Total time spent:                14:21:39.997000
Completed tasks:                 10
Incomplete tasks:                2

MILESTONE:  ci
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    0.5  hours
Maximum individual work time:    4.0  hours
Original estimate for complete:  4.75  →  57.0  hours
Total time spent:                14:21:39.997000
Completed tasks:                 10
Incomplete tasks:                2
(venv)
[nix-shell:~/pu/gradesta]$
```

Here, I only show output relevant to `ci` tasks. I can also do:

```
[nix-shell:~/pu/gradesta]$ kcf-tasks --milestone ci list-tasks
Could not decode file  /home/timothy/pu/gradesta/docs/gradesta-level-0.pdf
Could not decode file  /home/timothy/pu/gradesta/frontend/wireframes/gradesta-mobile.fig
Could not decode file  /home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/static/images/blog/publish-screencasts-flow.png
Could not decode file  /home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/static/images/favicon.png
Could not decode file  /home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/static/images/hero/hero-mask-svg.png
Could not decode file  /home/timothy/pu/gradesta/licenses/archive/AML/AML.xoj
Could not decode file  /home/timothy/pu/gradesta/protocol/gradesta-level-0.odt
0:30:00-4:00:00: Set up CI to test browser
DONE in 0:30:11.033000 estimated 0:00:00-0:00:00: Find a universal CI config format if it exists
DONE in 4:07:27.666000 estimated 1:00:00-16:00:00: Set up CI to test manager
DONE 0:30:00-4:00:00: Set up CI to run tests conditionally based on changes
DONE in 2:27:19.466000 estimated 0:15:00-1:00:00: Set up docker image with normal user in CI pipeline
DONE in 0:35:34.700000 estimated 0:15:00-1:00:00: Set up CI to do `cargo fmt`
DONE in 0:22:36.200000 estimated 0:15:00-1:00:00: Set up precommit hook to do `cargo fmt` everywhere
DONE in 0:27:05.933000 estimated 1:00:00-16:00:00: Figure out why the test `ageing_cellar::organize_sockets_dir::tests::test_socket_dir_old_socket` is flaky
DONE in 1:57:08.333000 estimated 1:00:00-16:00:00: Set up CI to show code coverage
DONE in 1:57:08.333000 estimated 0:15:00-1:00:00: Set up CI to test kcf code
DONE in 1:57:08.333000 estimated 0:15:00-1:00:00: Set up CI to ensure kcf code is black
```

So the whole reason I wanted to filter by milestone is actually so that I can generate JSON formatted lists of tasks which I can then graph using javascript. I still need one more set of filters. Filtering by complete vs incomplete tasks.

```
TASK: Filter by complete vs incomplete tasks
TASK_ID: 88df47824bd3b413574bff0dd647c400
CREATED: 2022-11-05 16:39
ESTIMATED_TIME: W1 DONE
MILESTONES: kcf-tasks/time-spent
```

```
TASK: JSON output from list-tasks command for graphing purposes
TASK_ID: 35905d199c7969fd463f8b03e93208bc
CREATED: 2022-11-05 16:54
ESTIMATED_TIME: W1 DONE
MILESTONES: kcf-tasks/time-spent
```

{{<screencast "2022-11-05-a58217ef-99c8-4169-8f65-34fd98d8dd9e" "46afe22b50d294cd8374fdc7293fc46c 88df47824bd3b413574bff0dd647c400">}}


Part 10: Subtasks(1) - milestone inheritance
-----------------------------------------------------

```
TASK: Subtasks
TASK_ID: 678e179e1987129076401cde6c3e5004
CREATED: 2022-11-05 18:34
ESTIMATED_TIME: U1 W4 DONE
MILESTONES: kcf-tasks/subtasks
```

So I thought for quite a bit about how to do sub tasks and 
wasn't able to find a better way than simply listing the task's parent.

This isn't perfect. Ergonomically it's annoying that you either have to have some special functionality in your editor or you need to spend time copying task parents around when breaking down tasks into subtasks.

When evaluating estimates, we will either take the parent task's estimate, or the sum of the child task's estimates, depending on which is larger. It should also be possible to provide no estimate for a parent task of course. Indeed, we can't provide estimates for tasks that are likely to take more than 256 hours in our system, and probably shouldn't make estimates for tasks that take more than 16 hours.

```
TASK: My Subtask                             # NO_TASK
TASK_ID: eaf6a0cf2f671ef5cd7932c4b177d60e    # NO_TASK
PARENT: 678e179e1987129076401cde6c3e5004     # NO_TASK
CREATED: 2022-11-05 18:42                    # NO_TASK
ESTIMATED_TIME: W2                           # NO_TASK
MILESTONES:  kcf-tasks/subtasks              # NO_TASK
```

(The NO_TASK directive prevents this example from being interpreted as an actual task...)

Question: Should MILESTONES be automatically inherited from the parent task? Almost certainly.

Question: Is this a DAG? Probably. In that case it should be `PARENTS` not `PARENT`. How does that effect time esitmates. How do the time estimates and time expenditures get divied up among parents? Probably by way of simple division. No, that actually doesn't make sense. If we want to know "how long until this feature will be complete" division won't help us. And task dependencies are not the same thing as sub-tasks, not by a long shot. Task dependencies are something different, almost every task depends on other tasks, but that shouldn't turn our todo list into a tree that extends rightwards rather than downwards... Making a DAG out of it and thinking of it as a dag would totally mess up the time estimates.

Maybe we can add `DEPENDENCIES` as well. Like make a DAG of tasks, but at the same time, be able to cut up tasks into subtasks. Cutting and connecting are, apparently two very different things when it comes to tasks, though not always is the distinction %100 clear cut.

```
TASK: Subtasks auto-inherit milestones from parent tasks
TASK_ID: 5bf3f2c74ac49bff9016e98b4eb42391
CREATED: 2022-11-05 18:53
ESTIMATED_TIME: W2 DONE
PARENT: 678e179e1987129076401cde6c3e5004
```

So we have a number of closely related but distinct concept when it comes to dependency and sub-tasks. Sometimes, a dependency is unrelated to the task at hand, other times the subtask is simply a rephrasing of the main task. Sometimes you have multiple alternative ways of doing something.

If I need to install OSB boards on my roof I can do it with a hammer and nails, a nail gun, screws and an electric screwdriver/impact driver/drill. So if I have the task install OSB boards, I might have 3 subtasks, "get hammer", "get nail gun", "get impact driver" and only one of them needs to be done, so we shouldn't sum these tasks, but rather choose the one that is best or easiest. In terms of estimation, if "get hammer" is a W2 and "get impact driver" is a W3 because we might have to charge it first, then our estimate range should be 15m to 4hours because that is the range of the fastest W2 to the slowest W3. But for now, we don't really have to deal with alternate tasks, it's just something to keep in mind for the future.

Back to milestone inheritance. We have an unsorted list of tasks. If we want to do milestone inheritance, we can do multiple passes, going through all child tasks over and over again untill there are no new milestones being inherited. We can put all the tasks into a tree/dag strucure and do a topological sort and do it then in one pass. We can put them into a tree structure and simply walk the tree, we can do event based inheritance in which the roots would fire a "READY" event to their children when they had inherited from their parents.

Or we can do a partial topological sort, putting tasks into layers based on how many parents they have. First pass we put in tasks with no parents, second pass we put in tasks who's parents are in the first pass. Well I guess that's only good for paralellism. I guess a normal toplogical sort is probably better, lets do that.

{{<screencast "2022-11-05-9c0add23-8f2b-40e7-8708-b969507a912c" "5bf3f2c74ac49bff9016e98b4eb42391">}}

Part 11: Subtasks (2) - passing on estimates from subtasks to parents
----------------------------------------------------------

```
TASK: Subtasks don't contribute to overall estimate but parents (in some cases) inherit time cost estimates from subtasks
TASK_ID: a39954d3e274dce726f6d212464137f6
CREATED: 2022-11-05 20:57
ESTIMATED_TIME: W3 DONE
PARENT: 678e179e1987129076401cde6c3e5004
```

If the sum of the time cost estimates for a parent task's subtasks is greater than the initial estimate for that parent, then the parent inherits the greater estimate.

In order to do this I've decided to modify the `ESTIMATED_TIME` field of the parent task by adding the sutbask estimates to the symbol list like so: ` | W2 W2 W2`. This allows for manual pseudo subtasks to be added directly in the estimate line like:

```
ESTIMATED_TIME: W3 | W2 W2 W2                              # NO_TASK
```

but this should be an uncommon usecase. In general, this is just a very convienient way to program. Rather than refactor my program, I do things POSIX style and make the user interface so powerfull that it can be used on programatically defined data and control panes as well. This type of coding has a *very* bad security track record, but I think it should be fine here, since we aren't actually executing anything that the user inputs.

{{<screencast "2022-11-05-99ebe816-228a-4b6c-b01c-8e79ffbe3aa8" "a39954d3e274dce726f6d212464137f6">}}

Part 12: Continuation of part 11
----------------------------------------

In part 11 I was thinking of using some kind of notation for the time estimates that would represent the time cost estimate tree but after further thought it seems that this is silly because this simply mirrors the task tree which we already liniked up. Copying the TCEs back through the tree just leads to duplicate data in the internal structures and the chance that user input would lead to weird notational syntax bugs that I have no desire to deal with. Instead, I'm going to do the TCE collation in the milestone estimation phase.

{{<screencast "2022-11-16-53f692b4-9e75-4344-97fd-6c74266fa4b1" "a39954d3e274dce726f6d212464137f6">}}

Part 13: Continuing to work on subtasks
-----------------------------------------------

{{<screencast "2022-11-20-749df679-596f-443a-8839-722679a982d0" "a39954d3e274dce726f6d212464137f6">}}


Part 14: Task graphing
--------------------------

So the general theory here is that a command like:

```
kcf-tasks --milestone=kcf-tasks/subtasks --json  list-tasks
```

Dumps out a json object like:

```
[
    {
        "BOUNTIED": null,
        "COMPLETED": null,
        "CREATED": "2022-11-05 18:34",
        "MAX_VALUE": null,
        "MILESTONES": [
            "all-tasks",
            "kcf-tasks/subtasks"
        ],
        "NAME": "Subtasks",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/008-time-estimates.md",
        "START_LINE_IN_SOURCE_FILE": 618,
        "START_VALUE": null,
        "TASK_ID": "678e179e1987129076401cde6c3e5004",
        "TASK_TIME_LOGs": [],
        "TIME_COST_ESTIMATES": [
            "U1",
            "W4",
            "DONE"
        ],
        "auto-describe-line": "DONE in 3:00:00 estimated 0:15:00-11:00:00: Subtasks 678e179e1987129076401cde6c3e5004"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-11-20 00:00",
        "CREATED": "2022-11-05 20:57",
        "MAX_VALUE": null,
        "MILESTONES": [
            "all-tasks",
            "kcf-tasks/subtasks"
        ],
        "NAME": "Subtasks don't contribute to overall estimate but parents (in some cases) inherit time cost estimates from subtasks",
        "PARENT": "678e179e1987129076401cde6c3e5004",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/008-time-estimates.md",
        "START_LINE_IN_SOURCE_FILE": 671,
        "START_VALUE": null,
        "TASK_ID": "a39954d3e274dce726f6d212464137f6",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs",
                "time_spent_seconds": 10800,
                "when": "2022-11-20 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W3",
            "DONE"
        ],
        "auto-describe-line": "DONE in 3:00:00 estimated 0:30:00-4:00:00: Subtasks don't contribute to overall estimate but parents (in some cases) inherit time cost estimates from subtasks a39954d3e274dce726f6d212464137f6"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": null,
        "CREATED": "2022-11-05 18:53",
        "MAX_VALUE": null,
        "MILESTONES": [
            "all-tasks",
            "kcf-tasks/subtasks"
        ],
        "NAME": "Subtasks auto-inherit milestones from parent tasks",
        "PARENT": "678e179e1987129076401cde6c3e5004",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/008-time-estimates.md",
        "START_LINE_IN_SOURCE_FILE": 650,
        "START_VALUE": null,
        "TASK_ID": "5bf3f2c74ac49bff9016e98b4eb42391",
        "TASK_TIME_LOGs": [],
        "TIME_COST_ESTIMATES": [
            "W2",
            "DONE"
        ],
        "auto-describe-line": "DONE 0:15:00-1:00:00: Subtasks auto-inherit milestones from parent tasks 5bf3f2c74ac49bff9016e98b4eb42391"
    }
]
```

This can then be put into a javascript var in the blog post and rendered using something like [charts.js](https://chartjs.org).

{{<screencast "2022-11-20-113fc380-6db5-4eb0-9350-a433af7bd795" "5ab6edc3f28466cb6bbfbb811bba78d3 35905d199c7969fd463f8b03e93208bc">}}

Part 15: Graphing continued - still in the planning stage
--------------------------------

It turns out that it is easy to say "I want to visualize the data and understand it" and somewhat harder to actually take that sentiment and decide what data should be graphed and how.

So far, it seems to me that the following graphs would be really usefull:

1. A graph of the time we've invested. It should show completed tasks over time, cumulatively. There are three lines, one showing the max amount of time it was estimated that those completed tasks would take. Another showing actual time. And a third showing the minimum estimate.

{{<pic "Time investment example graph" "/images/blog/008/time-invested.png">}}


2. The tasks remaining for each day. So we see how long the road in front of us is and how long it was.

{{<pic "Estimated time remaining example graph" "/images/blog/008/est-time-remaining.png">}}

3. A graph of how much time per day has been invested.

{{<strike>}}
Now it would be nice if each of these was a bar graph, one bar per day. The bars could be clicked on and a list of relevant tasks would be shown. Maybe there could be even a kind of spread chart or pie chart that would show how big an influence on the days total each task had.

In this part I've really just been planning what to graph and how. In the next part, hopefully I can go back to looking at javascript graphing libraries and figure out which one will work best for me.

It seems like I'll either have to use an old version of charts.js or find a different similar library. I need to be able to embed the charts into the blog posts but it seems like the most recent version of charts.js is meant to be compiled with NPM and no longer supports easy embedding (at least I couldn't find documentation on how to do so and found a few unanswered issues on gitub from people asking how...)

{{</strike>}}

(it turns out that I was totally wrong about this and there is a simple CDN compatible single js file version of chartjs)

{{<screencast "2022-11-24-ff3cae2c-6141-43f0-815d-064aa439cded" "5ab6edc3f28466cb6bbfbb811bba78d3">}}

Part 16: Setting up chartjs
---------------------------------

So I managed to get chartjs setup and running inline in the blog posts, but unfortunately, when you have a lot of datapoints in the horizontal access, things just get cut off at some point, no error, just incorrectly displayed data. Luckly, it appears that there is a plugin [chartjs-plugin-zoom](https://www.chartjs.org/chartjs-plugin-zoom/latest/guide/integration.html) that should hopefully fix this problem. In the next part, I'll try to get that one working.

{{<screencast "2022-11-27-89cb4903-9d27-4cd3-b6cf-73e78488e456" "5ab6edc3f28466cb6bbfbb811bba78d3">}} 


Part 17: Setting up chartjs-plugin-zoom
------------------------------------------------

So the plugin setup is pretty simple:

```
<script src="path/to/chartjs/dist/chart.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/hammerjs@2.0.8"></script>
<script src="path/to/chartjs-plugin-zoom/dist/chartjs-plugin-zoom.min.js"></script>
<script>
    var myChart = new Chart(ctx, {...});
</script>
```

I just need to put jammerjs on my own CDN and find chartjs-plugin-zoom.min.js . Maybe I can steal that from the network tab of my browser's debug tools. It is licensed MIT anyways. 

{{<rawhtmlexample>}}

<div class="chart-container" style="width:70vw;">
    <canvas id="panning-chart"></canvas>
</div>


<script src="https://assets.gradesta.com/gradesta/js/chart.js"></script>
<script src="https://assets.gradesta.com/gradesta/js/hammerjs.2.0.8.js"></script>
<script src="https://assets.gradesta.com/gradesta/js/chartjs-plugin-zoom.min.js"></script>

<script>
 const ctx = document.getElementById("panning-chart");

 const scaleOpts = {
     grid: {
         color: 'rgba( 0, 0, 0, 0.1)',
     },
     title: {
         display: true,
         text: (ctx) => ctx.scale.axis + ' axis',
     }
 };
 const scales = {
     x: {
         type: 'category',
         min: 5,
         max: 11,
     },
     y: {
         min: 0,
         max: 20,
     },
 };
 Object.keys(scales).forEach(scale => Object.assign(scales[scale], scaleOpts));
 
 new Chart(ctx, {
     type: 'bar',
     data: {
         "labels": ["start","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","Red", "Blue", "Yellow", "Green", "Purple", "Orange","end"],
         "datasets": [{
             "label": "# of Votes",
             "data": [12, 19, 3, 5, 2, 3,12, 19, 3, 5, 2, 3,12, 19, 3, 5, 2, 3,12, 19, 3, 5, 2, 3],
             "borderWidth": 1
         }]
},
     options: {
         scales: scales,
         plugins: {
             zoom: {
                 pan: {
                     enabled: true,
                     threshold: 14,
                     mode: "x",
                 }
     }}}
 });
</script>
{{</rawhtmlexample>}}

So after fighting with the imports and configs for a while (javascript provideded me with very little feedback on why things were not working), I figued out that I need to configure "scales", and now things are working.

{{<screencast "2022-11-28-728db710-5c46-4db8-9c08-1f746f317b77" "5ab6edc3f28466cb6bbfbb811bba78d3">}}

So now that I've figured out how to setup the graphs, in the next section I'll be writing some js to convert from kcf-tasks json task lists to graphjs data so we can display some data :)


Part 18: Converting kcf-task json lists to chartjs data for display
------------------------------------------------

So I started out just throwing a bunch of javascript pseudocode into an html file. When I realized that I had forgotten most javascript syntax, I decided to test my snippets by running them in a jupyter notebook using [ijavascript](https://github.com/n-riesco/ijavascript). Unfortunatelly, I found this to be unworkable because jupyter would throw errors if I ran the same code block twice:

```
evalmachine.<anonymous>:1
let earliest_relevant_date = null;
^

SyntaxError: Identifier 'earliest_relevant_date' has already been declared
    at Script.runInThisContext (vm.js:134:12)
    at Object.runInThisContext (vm.js:310:38)
    at run ([eval]:1020:15)
    at onRunRequest ([eval]:864:18)
    at onMessage ([eval]:828:13)
    at process.emit (events.js:400:28)
    at emit (internal/child_process.js:935:14)
    at processTicksAndRejections (internal/process/task_queues.js:83:21)
```

The only way to work around this was to restart the kernel. Not very good for quick prototyping/development.

Then I tried [RunKit](https://runkit.com/timthelion/638b6ab68e6345000864bdbb#), which worked well untill I had about 4 code blocks. Then the thing slowed to a crawl. Every time I pressed a key on the keyboard I had to wait around a second for it to respond.

So I decided to drop that tack and go with unit testing and TDD. I chose to try out [cypress](https://www.cypress.io/). After some initial configuration. I was happy to see that unit tests were quite easy to write, and the in browser test runner is (mostly) a joy to use.

I relatively quickly got to the point where most of my application code was working. Now for the fun part. Cypress supports in browser integration tests too! :O

Unfortunatelly, it wasn't nearly so easy to figure out how to get those working. The test suit looks pretty easy to use, but the very first example looks like this:

```
it('adds todos', () => {
  cy.visit('https://todo.app.com')
  cy.get('[data-testid="new-todo"]')
    .type('write code{enter}')
    .type('write tests{enter}')
  // confirm the application is showing two items
  cy.get('[data-testid="todos"]').should('have.length', 2)
})
```

The thing is, I didn't want to test `todo.app.com`. Nor do I want to upload my code to some web server. I want to test my code Locally, Instantly, As I write it. In other examples, they show accessing localhost. But no where do I see them launching the web server. Does this mean that they just want you to launch the web server BEFORE launching the test suit? Weird...

Maybe that IS what they expect you to do though. I guess I need a web server that automatically updates as the code is edited and serves the current code. They don't really explain that [in the docs linked on the home page](https://docs.cypress.io/guides/overview/why-cypress#Cypress-in-the-Real-World). But I'm just going to assume for now, that that's the case, and configure my CI to launch the server first. Indeed, that appears to be best practice. Just run your own web server. In the official Cypress "Real World App" example they do it [like this](https://github.com/cypress-io/cypress-realworld-app/blob/a221bdb42e49f52bb07944cee9b9dff3644c3bb4/.github/workflows/main.yml#L98) in ci:

```
      - name: "UI Tests - Chrome"
        uses: cypress-io/github-action@v4
        with:
          start: yarn start:ci
          wait-on: "http://localhost:3000"
          wait-on-timeout: 120
          browser: chrome
          record: true
          parallel: true
          group: "UI - Chrome"
          spec: cypress/tests/ui/*
          config-file: cypress.config.js
        env:
          CYPRESS_PROJECT_ID: ${{ secrets.CYPRESS_PROJECT_ID }}
          CYPRESS_RECORD_KEY: ${{ secrets.CYPRESS_RECORD_KEY }}
          # Recommended: pass the GitHub token lets this action correctly
          # determine the unique run id necessary to re-run the checks
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          DEBUG: "cypress:server:args"
```

Apparently the paid web service associated with cypress allows you to easilly view the screenshots that are collected during the tests.

Here is what [`start:ci`](https://github.com/cypress-io/cypress-realworld-app/blob/a221bdb42e49f52bb07944cee9b9dff3644c3bb4/package.json#L143) does in their instance:

```
    "start:ci": "cross-env NODE_ENV=test concurrently yarn:start:react:proxy-server yarn:start:api",
```

In my case I'll probably just use a basic node server. I don't need any of that fancy react stuff for putting a few charts in a blog.

Before doing that I just wanted to finish setting up the project, so I configured [eslint](https://eslint.org/docs/latest/user-guide/command-line-interface#fixing-problems) to format my code as well.

This screencast part is already almost 8 hours long though. So I decided to break this task up into subtasks and record another part.

{{<screencast "2022-12-01-1abb0c39-d5e3-4cdb-8c1a-a26d2250a823" "5ab6edc3f28466cb6bbfbb811bba78d3">}}


Part 19: Graphing roadmap
------------------------------

So I need to break up task `5ab6edc3f28466cb6bbfbb811bba78d3` into some subtasks before I continue. It's just way bigger than I somehow stupidly assumed. I still need to:

```
TASK: Configure cypress to run web server and capture and test estimated_time_remaining graph
TASK_ID: 1ae984a704ea6cd346d56adfeaafec85
CREATED: 2022-12-04 07:04
ESTIMATED_TIME: W4
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

```
TASK: Implement dataset gathering functions for time_invested graph
TASK_ID: 91ab0076ae3ec0b6984e7705d2d3fc09
CREATED: 2022-12-04 07:04
ESTIMATED_TIME: W3
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

```
TASK: Ensure graphs work correctly for tasks that were created and completed on same day
TASK_ID: 9fde3b27d52b472d9c5f26bd91582132
CREATED: 2022-12-04 07:05
ESTIMATED_TIME: W2
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

```
TASK: Implement visual tests for time invested graph
TASK_ID: 242024c6668b19938449b7f3d7d36777
CREATED: 2022-12-04 07:06
ESTIMATED_TIME: W2
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

```
TASK: Integrate graphs into hugo shortcode
TASK_ID: 58653d78a451f3a49a43a1caaadc2c31
CREATED: 2022-12-04 07:06
ESTIMATED_TIME: W2
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

```
TASK: Implement storing JSON for task graph outside of blog post but make it still simple to use shortcode
TASK_ID: c92b94b0ad5cbc11e70ce0a2f68b16a8
CREATED: 2022-12-04 07:08
ESTIMATED_TIME: W3
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

So the new estimate looks like this. Looks like I'll be done in somewhere between 3 and 31 hours (if my estimates are worth anything which they probably aren't).

```
MILESTONE:  kcf-tasks/time-spent
Minimum decision time:           0:00:00
Maximum decision time:           0:00:00
Minimum team work time:          0.0  hours
Maximum team work time:          0.0  hours
Minimum individual work time:    3.25  hours
Maximum individual work time:    31.0  hours
Original estimate for complete:  3.6666666666666665  →  27.5  hours
Total time spent:                0:00:00
Completed tasks:                 10
Incomplete tasks:                8
```

Note that the total time spent will only be filled in once I upload the screencasts.

{{<screencast "2022-12-04-05647097-0cae-4358-90e7-bdf8d4090f15" "5ab6edc3f28466cb6bbfbb811bba78d3">}}

Part 20: Configuring webpack
-------------------------------------------------------------------------

In part 18 I examined how to work with cypress's integration/e2e tests. For these, I need to run a webserver to display the charts. I will probably do this by creating a new tiny web app in the kcf_graph examples directory. This will allow my test case to also inform users on how to use the library. I guess the first thing I need to do is pack/build the kcf_graph library using webpack. This actually becomes a task of its own. IIRC, just setting up webpack can be a significant ordeal.

```
TASK: Pack/build kcf_graph with webpack
TASK_ID: 9b640aff70b4b48b99106f4f3299e9c6
CREATED: 2022-12-04 07:36
ESTIMATED_TIME: W3 DONE
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

So you'd think this would be easy. They say you don't even need a config, though I ended up having to write one anyways:

```
const path = require('path');

module.exports = {
    entry: './src/kcf-graph.js',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'kcf-graph.js',
    },
};
```

Running

```
$ npx webpack
asset kcf-graph.js 202 KiB [compared for emit] [minimized] (name: main) 1 related asset
orphan modules 442 KiB [orphan] 5 modules
runtime modules 663 bytes 3 modules
cacheable modules 517 KiB
  ./src/kcf-graph.js + 5 modules 445 KiB [built] [code generated]
  ./node_modules/hammerjs/hammer.js 72.1 KiB [built] [code generated]

WARNING in configuration
The 'mode' option has not been set, webpack will fallback to 'production' for this value.
Set 'mode' option to 'development' or 'production' to enable defaults for each environment.
You can also set it to 'none' to disable any default behavior. Learn more: https://webpack.js.org/configuration/mode/

webpack 5.75.0 compiled with 1 warning in 2194 ms
```

Returns seemingly with no errors. But the generated 200+Kb `dist/kcf-graph.js` file does not contain any of my code. No, it's not obfuscated or twisted, it's just not there. This is why I continue to bang my head against those rust lifetime errors. At least `cargo build` includes my code in the executable. Anyhow... Luckly we have [stack overflow](https://stackoverflow.com/questions/56102640/how-to-export-a-function-with-webpack-and-use-it-in-an-html-page). It was enough to add the `library: "libraryName"` property to the output object in the webpack config.

```
TASK: Show relevant tasks when mouse is over point in graph
TASK_ID: 5de1af83fb9a703d080e94c8686e1820
CREATED: 2022-12-04 08:43
ESTIMATED_TIME: W3
PARENT: 5ab6edc3f28466cb6bbfbb811bba78d3
```

{{<screencast "2022-12-04-0734e900-fff9-48aa-8ba3-2cc612d123ad" "9b640aff70b4b48b99106f4f3299e9c6">}}

Part 21: Showing task lists when clicking on point in graph
-----------------------------------------------------------------------

{{<screencast "2022-12-04-3459ecb6-8e70-4442-b114-08e1b4b87b6d" "5de1af83fb9a703d080e94c8686e1820">}}

{{<tasktimegraph>}}
[
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-23 00:00",
        "CREATED": "2022-09-01 18:53",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up CI to ensure kcf code is black",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 180,
        "START_VALUE": null,
        "TASK_ID": "0f0a4c7c0df8a6683b9f292c3cc0c5f5",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 7028,
                "when": "2022-10-23 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W2",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 3600,
            "individual_work_estimated_completed_min": 900,
            "individual_work_max": 3600,
            "individual_work_min": 900,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 1:57:08.333000 estimated 0:15:00-1:00:00: Set up CI to ensure kcf code is black 0f0a4c7c0df8a6683b9f292c3cc0c5f5"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-23 00:00",
        "CREATED": "2022-09-01 18:53",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up CI to test kcf code",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 174,
        "START_VALUE": null,
        "TASK_ID": "216868ec2a5f6adf295dd6688737c56c",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 7028,
                "when": "2022-10-23 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W2",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 3600,
            "individual_work_estimated_completed_min": 900,
            "individual_work_max": 3600,
            "individual_work_min": 900,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 1:57:08.333000 estimated 0:15:00-1:00:00: Set up CI to test kcf code 216868ec2a5f6adf295dd6688737c56c"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-23 00:00",
        "CREATED": "2022-09-01 18:52",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up CI to show code coverage",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 167,
        "START_VALUE": null,
        "TASK_ID": "47c7ff403b446e8b42a87401d35fd450",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 7028,
                "when": "2022-10-23 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W4",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 57600,
            "individual_work_estimated_completed_min": 3600,
            "individual_work_max": 57600,
            "individual_work_min": 3600,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 1:57:08.333000 estimated 1:00:00-16:00:00: Set up CI to show code coverage 47c7ff403b446e8b42a87401d35fd450"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-23 00:00",
        "CREATED": "2022-10-23 16:01",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Figure out why the test `ageing_cellar::organize_sockets_dir::tests::test_socket_dir_old_socket` is flaky",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 140,
        "START_VALUE": null,
        "TASK_ID": "a70acc872494bb716e620fa735fd8eed",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 1625,
                "when": "2022-10-23 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W4",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 57600,
            "individual_work_estimated_completed_min": 3600,
            "individual_work_max": 57600,
            "individual_work_min": 3600,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 0:27:05.933000 estimated 1:00:00-16:00:00: Figure out why the test `ageing_cellar::organize_sockets_dir::tests::test_socket_dir_old_socket` is flaky a70acc872494bb716e620fa735fd8eed"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-23 00:00",
        "CREATED": "2022-09-01 18:51",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up precommit hook to do `cargo fmt` everywhere",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 124,
        "START_VALUE": null,
        "TASK_ID": "f7b43334d359dd3d2aa47c3c28fbece4",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 1356,
                "when": "2022-10-23 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W2",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 3600,
            "individual_work_estimated_completed_min": 900,
            "individual_work_max": 3600,
            "individual_work_min": 900,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 0:22:36.200000 estimated 0:15:00-1:00:00: Set up precommit hook to do `cargo fmt` everywhere f7b43334d359dd3d2aa47c3c28fbece4"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-23 00:00",
        "CREATED": "2022-09-01 18:51",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up CI to do `cargo fmt`",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 109,
        "START_VALUE": null,
        "TASK_ID": "c4cea87b7e9a0db374d6679570555e08",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 1356,
                "when": "2022-10-23 00:00"
            },
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 778,
                "when": "2022-10-19 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W2",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 3600,
            "individual_work_estimated_completed_min": 900,
            "individual_work_max": 3600,
            "individual_work_min": 900,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 0:35:34.700000 estimated 0:15:00-1:00:00: Set up CI to do `cargo fmt` c4cea87b7e9a0db374d6679570555e08"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-16 00:00",
        "CREATED": "2022-09-20 19:06",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up docker image with normal user in CI pipeline",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 65,
        "START_VALUE": null,
        "TASK_ID": "8886c40d54bf08d3ef40ae5d7207ebf6",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 8839,
                "when": "2022-10-16 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W2",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 3600,
            "individual_work_estimated_completed_min": 900,
            "individual_work_max": 3600,
            "individual_work_min": 900,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 2:27:19.466000 estimated 0:15:00-1:00:00: Set up docker image with normal user in CI pipeline 8886c40d54bf08d3ef40ae5d7207ebf6"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": null,
        "CREATED": "2022-09-01 18:52",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up CI to run tests conditionally based on changes",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 46,
        "START_VALUE": null,
        "TASK_ID": "8c2b82fd591898b1807aeee26c793d7e",
        "TASK_TIME_LOGs": [],
        "TIME_COST_ESTIMATES": [
            "W3",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 14400,
            "individual_work_estimated_completed_min": 1800,
            "individual_work_max": 14400,
            "individual_work_min": 1800,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE 0:30:00-4:00:00: Set up CI to run tests conditionally based on changes 8c2b82fd591898b1807aeee26c793d7e"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-10-16 00:00",
        "CREATED": "2022-09-01 18:49",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up CI to test manager",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 40,
        "START_VALUE": null,
        "TASK_ID": "e59db446d76a2ab972abb1bfab616376",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 8839,
                "when": "2022-10-16 00:00"
            },
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 6008,
                "when": "2022-09-18 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "W4",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 57600,
            "individual_work_estimated_completed_min": 3600,
            "individual_work_max": 57600,
            "individual_work_min": 3600,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 4:07:27.666000 estimated 1:00:00-16:00:00: Set up CI to test manager e59db446d76a2ab972abb1bfab616376"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": "2022-09-18 00:00",
        "CREATED": "2022-09-18 19:11",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Find a universal CI config format if it exists",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/gradesta-s.r.o.-commercial-stuff/web/content/blog/007-ci.md",
        "START_LINE_IN_SOURCE_FILE": 21,
        "START_VALUE": null,
        "TASK_ID": "afdb8ee2feb8be6db7925837828dc2c0",
        "TASK_TIME_LOGs": [
            {
                "author": "Timothy Hobbs <tim@gradesta.com>",
                "time_spent_seconds": 1811,
                "when": "2022-09-18 00:00"
            }
        ],
        "TIME_COST_ESTIMATES": [
            "U1",
            "DONE"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 14400,
            "decision_min": 900,
            "individual_work_estimated_completed_max": 0,
            "individual_work_estimated_completed_min": 0,
            "individual_work_max": 0,
            "individual_work_min": 0,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE in 0:30:11.033000 estimated 0:00:00-0:00:00: Find a universal CI config format if it exists afdb8ee2feb8be6db7925837828dc2c0"
    },
    {
        "BOUNTIED": null,
        "COMPLETED": null,
        "CREATED": "2022-09-01 18:52",
        "MAX_VALUE": null,
        "MILESTONES": [
            "mvp",
            "all-tasks",
            "ci"
        ],
        "NAME": "Set up CI to test browser",
        "PARENT": "",
        "SOURCE_FILE": "/home/timothy/pu/gradesta/blackhole/feature/roadmap/THE_BIG_LIST.org",
        "START_LINE_IN_SOURCE_FILE": 7,
        "START_VALUE": null,
        "TASK_ID": "dd380b60bc0085acdb079403646ff9f9",
        "TASK_TIME_LOGs": [],
        "TIME_COST_ESTIMATES": [
            "W3"
        ],
        "TIME_COST_ESTIMATES_SUMMARY": {
            "decision_max": 0,
            "decision_min": 0,
            "individual_work_estimated_completed_max": 0,
            "individual_work_estimated_completed_min": 0,
            "individual_work_max": 14400,
            "individual_work_min": 1800,
            "team_work_max": 0,
            "team_work_min": 0
        },
        "auto-describe-line": "DONE 0:30:00-4:00:00: Set up CI to test browser dd380b60bc0085acdb079403646ff9f9"
    }
]
{{</tasktimegraph>}}
