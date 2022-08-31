Estimating time
-------------------

There are various types of time when it comes to the development process.

In our case we are interested in the following two:

- Decision making time
- Coding time

Decisions
-----------

Decision making time is both active (research) and passive. It isn't something that can be recorded on video or easilly measured. A good nights sleep or a walk in the park are as important investments to good decision making as an hour spent reading various documents/marketing copy ect. That said, it is important to destinguish between the investment one makes into reasearch, brain storming and deep thought, and documenting the decision. Docoumeting the research and the decision is an important and often overlooked stage of decision making.

Mark decision tasks with the prefix `U` for "unknown" and a number classifying how "big" the decision is.

```
TIME_COST: U1
```

Since decision making requires "passive" time like sleep as well as active time, we measure decisions in days, weeks and months rather than "person hours".

- `U0`: max 15 min
- `U1`: 15min to 4 hours
- `U2`: 2hours to 2 days
- `U3`: 1 day to 1 week
- `U4`: 3 days to 2 weeks
- `U5`: 1 week to 1 month
- `U6`: 2 weeks to 3 months
- `U7`: 2 months to 6 months
- `U8`: 4 months to 1 year

Work
-----

The most important property of "work" is knowing whether it can be paralalelized. We classify work tasks in terms of person hours in the format of `W<scale-of-non-paraleliziable-person-hours>` and/or `T<scale-of-paralelizable-person-hours>`. 

Individual tasks:

- `W1` - 5 minutes to 45 minutes
- `W2` - 15 minutes to 1 hour
- `W3` - 30 minutes to 4 hours
- `W4` - 1 hour to 16 hours
- `W5` - 4 hours to 32 hours
- `W6` - 8 hours to 64 hours
- `W7` - 16 hours to 128 hours
- `W8` - 32 hours to 256 hours

Team tasks:

- `T1` - 10 hours to 20 hours
- `T2` - 15 hours to 30 hours
- `T3` - 20 hours to 40 hours
- `T4` - 30 hours to 60 hours
- `T5` - 40 hours to 80 hours
- `T6` - 60 hours to 120 hours
- `T7` - 80 hours to 160 hours
- `T8` - 100 hours to 200 hours
- `T9` - 150 hours to 300 hours
- `T10` - 200 hours to 400 hours
- `T11` - 300 hours to 800 hours
- `T12` - 600 hours to 1200 hours

Classifying tasks in terms of time cost
-------------------------

Only look at the maximum time, not the minimums. Go from the top of the list down and for each minimum, ask yourself, "Is this task pretty much gauranteed to be completed by the maximum allowed time? If so, use that classification. Don't spend too much time classifying each task, The classification process itself should be a `U0`.
