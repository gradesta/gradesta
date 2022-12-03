kcf-graph
-----------

Graph your tasks over time, inline in your static or dynamic blog or web page.

kcf-graph is a simple javascript library that allows you to take the output of the

```
kcf-tasks --json list_tasks
```

command and graph those tasks using [chart.js](https://www.chartjs.org/).

To use simply do:

```
<script src="https://assets.gradesta.com/gradesta/js/chart.js"></script>
<script src="https://assets.gradesta.com/gradesta/js/hammerjs.2.0.8.js"></script>
<script src="https://assets.gradesta.com/gradesta/js/chartjs-plugin-zoom.min.js"></script>
<script src="https://assets.gradesta.com/gradesta/js/kcf-graph.js"></script>

...

<div class="chart-container" style="width:70vw;">
    <span style="text-align: center"> Estimated time remaining </span>
    <canvas id="someId"></canvas>
</div>

<div class="chart-container" style="width:70vw;">
    <span style="text-align: center"> Time invested vs estimates </span>
    <canvas id="anotherId"></canvas>
</div>


<script>

const remaining_canvas = document.getElementById("someId");
const invested_canvas = document.getElementById("onotherId");
let tasks = <output of: kcf-tasks --json list_tasks>
estimated_time_remaining(remaining_canvas, tasks);
time_invested_vs_estimate(invested_canvas, tasks);
</script>
```
