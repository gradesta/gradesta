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
<script src="https://assets.gradesta.com/gradesta/js/kcf-graph.js"></script>

...

<div class="chart-container" style="width:70vw;">
    <span style="text-align: center"> Estimated time remaining </span>
    <canvas id="someId"></canvas>
</div>

<div class="chart-container" style="width:70vw;">
    <span style="text-align: center"> Time invested vs estimates </span>
    <div id="tooltip"></div>
    <canvas id="anotherId"></canvas>
</div>


<script>

const remaining_canvas = document.getElementById("someId");
const invested_canvas = document.getElementById("onotherId");
const tooltipEl = document.getElementById("tooltip");
let tasks = <output of: kcf-tasks --json list_tasks>
kcfGraph.estimated_time_remaining(remaining_canvas, tooltipEl, tasks);
kcfGraph.time_invested_vs_estimate(invested_canvas, tooltipEl, tasks);
</script>
```

Development
--------------

Build with (needed to get examples to work)

```
npx webpack
```

test with

```
npm run cypress:open
```

you can play with examples by loading the webserver

```

```

And visiting

```
http://localhost:<port>/examples/index.html
```
