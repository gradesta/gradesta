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
ESTIMATED_TIME: W3
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

after that, I returned to the task involving milestone assessment.


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
