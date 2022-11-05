---
title: "Analyzing the quality of time estimates and actual time spent"
date: 2022-10-25
featureimage: https://assets.gradesta.com/gradesta/img/dalle2-clock-and-coins.png
author: timothy hobbs <tim@gradesta.com>
draft: true
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
ESTIMATED_TIME: U1 W4
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
ESTIMATED_TIME: W2
PARENT: 678e179e1987129076401cde6c3e5004
```

So we have a number of closely related but distinct concept when it comes to dependency and sub-tasks. Sometimes, a dependency is unrelated to the task at hand, other times the subtask is simply a rephrasing of the main task. Sometimes you have multiple alternative ways of doing something.

If I need to install OSB boards on my roof I can do it with a hammer and nails, a nail gun, screws and an electric screwdriver/impact driver/drill. So if I have the task install OSB boards, I might have 3 subtasks, "get hammer", "get nail gun", "get impact driver" and only one of them needs to be done, so we shouldn't sum these tasks, but rather choose the one that is best or easiest. In terms of estimation, if "get hammer" is a W2 and "get impact driver" is a W3 because we might have to charge it first, then our estimate range should be 15m to 4hours because that is the range of the fastest W2 to the slowest W3. But for now, we don't really have to deal with alternate tasks, it's just something to keep in mind for the future.

Back to milestone inheritance. We have an unsorted list of tasks. If we want to do milestone inheritance, we can do multiple passes, going through all child tasks over and over again untill there are no new milestones being inherited. We can put all the tasks into a tree/dag strucure and do a topological sort and do it then in one pass. We can put them into a tree structure and simply walk the tree, we can do event based inheritance in which the roots would fire a "READY" event to their children when they had inherited from their parents.

Or we can do a partial topological sort, putting tasks into layers based on how many parents they have. First pass we put in tasks with no parents, second pass we put in tasks who's parents are in the first pass. Well I guess that's only good for paralellism. I guess a normal toplogical sort is probably better, lets do that.

{{<screencast "2022-11-05-9c0add23-8f2b-40e7-8708-b969507a912c" "5bf3f2c74ac49bff9016e98b4eb42391">}}
