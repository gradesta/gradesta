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
MILESTONES: kcf-task-management-time-spent
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
MILESTONES: kcf-task-management-time-spent
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
MILESTONES: kcf-task-management-time-spent
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
MILESTONES: kcf-task-management-time-spent
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
MILESTONES: kcf-task-management-time-spent
```

however, half way through it occured to me that I already showed actual time spend next to milestones. I didn't realize that I needed to also show previously estimated time spend for already completed tasks to make a reasonbable comparison.

After that, I did

```
*** TASK: Listing completed tasks with actual vs estimated time spend
TASK_ID: ce205fc194d577549e245aaa8a09d6cc
CREATED: 2022-11-05 11:57
ESTIMATED_TIME: W3 DONE
MILESTONES: kcf-task-management-time-spent
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

MILESTONE:  kcf-task-management-overhead-tasks
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

MILESTONE:  kcf-task-management-time-spent
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


{{<screencast "2022-11-05-959d265f-7ae6-4c3f-bfa4-1802ab819576" "ce205fc194d577549e245aaa8a09d6cc 31a55ad0bb8f87121a23ff95c81fe558">}}

Part 8: Creating a table to compair time estimate types vs actual time spend
---------------------------------------------------------

This will let me find out how long each type actually takes so that I can see if I'm any good at estimates or not.

```
TASK: Listing estimate type by actual time spend
TASK_ID: 5096c557109aa1292c40b00b39d76518
CREATED: 2022-11-05 11:58
ESTIMATED_TIME: W3
MILESTONES: kcf-task-management-time-spent

So it would print a table like:

      estimate         actual average       datapoints
W1     5min - 45 min        37 min              3
W2     15min - 1 hour       45 min              7
......
```
