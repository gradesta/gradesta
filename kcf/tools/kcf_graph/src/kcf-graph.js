import { Chart, CategoryScale, LinearScale, LineController, LineElement, PointElement } from 'chart.js'
import zoomPlugin from 'chartjs-plugin-zoom'
import { mk_scales } from "./scales.js"

Chart.register(zoomPlugin)
Chart.register(CategoryScale)
Chart.register(LinearScale)
Chart.register(LineController)
Chart.register(LineElement)
Chart.register(PointElement)


export function get_date_range (tasks) {
  let earliest_relevant_date = null
  let latest_relevant_date = null
  for (const task of tasks) {
    if (earliest_relevant_date === null) {
      earliest_relevant_date = task.CREATED
    }
    if (task.COMPLETED) {
      if (latest_relevant_date === null) {
        latest_relevant_date = task.COMPLETED
      }
      if (task.COMPLETED > latest_relevant_date) {
        latest_relevant_date = task.COMPLETED
      }
    }
    if (task.CREATED < earliest_relevant_date) {
      earliest_relevant_date = task.CREATED
    }
  }

  return { start: earliest_relevant_date, end: latest_relevant_date }
}

// Gets a list of strings representing the dates between the start and end dates
// plus two days extra to give context to the graph
export function get_dates_as_ms (earliest_relevant_date, latest_relevant_date) {
  const dates = []
  let day = Date.parse(earliest_relevant_date.split(' ')[0])
  const latest_day = Date.parse(latest_relevant_date.split(' ')[0])
  while (day <= (latest_day + 2 * 1000 * 60 * 60 * 24)) {
    dates.push(day)
    day += 1000 * 60 * 60 * 24
  }
  return dates
}

export function get_dates_as_strings (earliest_relevant_date, latest_relevant_date) {
  return get_dates_as_ms(earliest_relevant_date, latest_relevant_date).map(date => {
    const d = new Date(date)
    return d.getFullYear() + '-' + ('0' + (d.getMonth() + 1)).slice(-2) + '-' + ('0' + d.getDate()).slice(-2)
  })
}

// A day by day series of estimates of the number of hours remaining
export function get_estimates_hours (tasks, dates) {
  const min_estimates_hours = []
  const max_estimates_hours = []
  for (const date of dates) {
    let min_estimate_sec = 0
    let max_estimate_sec = 0
    for (const task of tasks) {
      if (task.CREATED <= date) {
        if (task.COMPLETED <= date) {
          continue
        }
        min_estimate_sec += task.TIME_COST_ESTIMATES_SUMMARY.individual_work_min
        max_estimate_sec += task.TIME_COST_ESTIMATES_SUMMARY.individual_work_max
      }
    }
    min_estimates_hours.push(min_estimate_sec / 60 / 60)
    max_estimates_hours.push(max_estimate_sec / 60 / 60)
  }
  return { min: min_estimates_hours, max: max_estimates_hours }
}

export const panning_zoom_options = {
  zoom: {
    pan: {
      enabled: true,
      threshold: 14,
      mode: 'x'
    }
  }
}

export function estimated_time_remaining (element, tooltipEl, tasks) {
  const date_range = get_date_range(tasks)
  const labels = get_dates_as_strings(date_range.start, date_range.end)
  const estimated_hours = get_estimates_hours(tasks, labels)
  let max_hours = 0
  for (const duration of estimated_hours.max) {
    if (duration > max_hours) {
      max_hours = duration
    }
  }
  const scales = mk_scales()
  scales.y.max = max_hours + 4
  const chart = new Chart(element, {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'min estimated hours',
          data: estimated_hours.min
        },
        {
          label: 'max estimated hours',
          data: estimated_hours.max
        }
      ]
    },
    options: {
      scales,
      plugins: panning_zoom_options
    }
  })

  function clickHandler (evt) {
    const points = chart.getElementsAtEventForMode(evt, 'nearest', { intersect: true }, true)

    if (points.length) {
      const firstPoint = points[0]
      const label = chart.data.labels[firstPoint.index]
      let tooltip = '<list>'
      for (const task of tasks) {
        if (task.CREATED <= label) {
          if (task.COMPLETED == null || task.COMPLETED > label) {
            tooltip += '<li>' + task['auto-describe-line'] + '</li>'
          }
        }
      }
      tooltipEl.innerHTML = tooltip + '</list>'
    }
  }

  element.onclick = clickHandler
  return chart
}

/*
   A Task object looks like this:

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

    We want to go through the tasks, and the TASK_TIME_LOGs, and sort them by day.

    So for each day we want to have a list of TIME_LOGs.

    Then we want to create time investment totals per day.

*/
export function time_invested_per_day (tasks) {
  const date_range = get_date_range(tasks)
  const labels = get_dates_as_strings(date_range.start, date_range.end)
  const time_invested_per_day = {}
  for (const label of labels) {
    time_invested_per_day[label] = {
      time_invested: 0,
      tasks: []
    }
  }

  for (const task of tasks) {
    for (const time_log of task.TASK_TIME_LOGs) {
      const date = time_log.when.split(' ')[0]
      time_invested_per_day[date].time_invested += time_log.time_spent_seconds
      time_invested_per_day[date].tasks.push(task)
    }
  }
  return time_invested_per_day
}

// Calculate max hours from max time estimated for completed tasks and max time invested
export function max_hours_invested_or_estimated (tasks, days, tipd) {
  let max_hours = 0
  for (const task of tasks) {
    if (task.COMPLETED != null) {
      max_hours += task.TIME_COST_ESTIMATES_SUMMARY.individual_work_max / 60 / 60
    }
  }
  let total_time_invested = 0
  for (const day of days) {
    total_time_invested = tipd[day].time_invested / 60 / 60
  }
  if (total_time_invested > max_hours) {
    max_hours = total_time_invested
  }
  return max_hours
}

export function time_invested_vs_estimate (element, tooltipEl, tasks) {
  const date_range = get_date_range(tasks)
  const labels = get_dates_as_strings(date_range.start, date_range.end)
  const tipd = time_invested_per_day(tasks)
  const max_hours = max_hours_invested_or_estimated(tasks, labels, tipd)
  const scales = mk_scales()
  scales.y.max = max_hours + 4
  const chart = new Chart(element, {
    type: 'line',
    data: {
      labels,
      datasets: [
        {
          label: 'time invested each day',
          data: labels.map(label => tipd[label].time_invested / 60 / 60)
        },
        {
          label: 'time invested total',
          data: labels.map(label => {
            let total = 0
            for (const label2 of labels) {
              total += tipd[label2].time_invested / 60 / 60
              if (label2 === label) {
                break
              }
            }
            return total
          })
        }
      ]
    },
    options: {
      scales,
      plugins:
        panning_zoom_options
    }
  })

  function clickHandler (evt) {
    const points = chart.getElementsAtEventForMode(evt, 'nearest', { intersect: true }, true)

    if (points.length) {
      const firstPoint = points[0]
      const label = chart.data.labels[firstPoint.index]
      let tooltip = '<list>'
      for (const task of tipd[label].tasks) {
        tooltip += '<li>' + task['auto-describe-line'] + '</li>'
      }
      tooltipEl.innerHTML = tooltip + '</list>'
    }
  }

  element.onclick = clickHandler
  return chart
}
