Determining priority
-------------------------

A task has a priority that is determined by the dollars per minute/hour/day that it costs to not have that task done.

So if I have a task (task 1) that costs $0.001 per minute and a task (task 2) that costs $1 per minute than the $1 per minute task is of greater priority. However, that doesn't mean it never makes sense to do the $0.001 per minute task. If that task ends up remaining relevant for a long time, it would end up costing a lot of money. It is also true, that seemingly important tasks (tasks that cost a lot per minute to not have compileted), may become quickly irrelevant as market conditions/technology change. So a $1 per minute task that remians relevant for just 5 minutes is less important in the long run, than a $0.001 per minute task that reminains relevant for years. So it makes sense, to measure task priority in terms of accumulated costs. We could try to guess which tasks were both likely to bring a lot of value and stay relevant for a long time, but this is (I'm guessing) quite difficult. So task priority is determined in our system, by the accumulated estimated cost during the time that it is incomplete.

This accumulated cost, can also be used to determine the price we should pay a subcontractor to complete the task. In a public tender system, an easy to complete ask will be done quickly, where-as a hard to do task will be done slowly. Therefore, it is logical, to use the accumulated "task value" to determine the price that should be paid to a contractor.

Since we don't want to wait some arbitrarilly long time before contractors will even touch tasks that are slowly accumulating value, it is prudent to calcualte the "task value", from the accumulated cost PLUS some arbitrary "starting cost" that we are sure is "slightly less than reasonable", to jump start the "task value" accumulation. So a $1 per minute task might have a "start value" of $50 so we don't have to needlessy wait 50 minutes for contractors to become interested in them. It is also necisary to have a "maximum value" associated with tasks, so that it is possible to create a fixed budget and ensure that there are not wild unknown budget over-runs. The maximum value does not need to be fixed, it can be updated manually for tasks that become stagnant and are never completed.

So overall, a task priority might look as follows:

```
Task 1:
INCOMPLETION_COST: $1 per hour
START_VALUE: $50
MAX_VALUE: $5000
PUBLISHED: 2022-08-29 10:40
Description: ....
```

To calculate the current value for this task, one can take the `TASK_VALUE = min(START_VALUE + (NOW - PUBLISHED) * INCOMPLETION_COST, MAX_VALUE)`.
