export const scales = {
    x: {
        type: 'category',
        min: 5,
        max: 11,
        grid: {
            color: 'rgba( 0, 0, 0, 0.1)',
        },
        title: {
            display: true,
            text: (ctx) => ctx.scale.axis + ' axis',
        }
    },
    y: {
        min: 0,
        max: 20,
        grid: {
            color: 'rgba( 0, 0, 0, 0.1)',
        },
        title: {
            display: true,
            text: (ctx) => ctx.scale.axis + ' axis',
        }
    },
}

export function get_date_range(tasks) {
    let earliest_relevant_date = null;
    let latest_relevant_date = null;
    for (const task of tasks) {
        if (earliest_relevant_date === null) {
            earliest_relevant_date = task.CREATED;
        }
        if (task.COMPLETED) {
            if (latest_relevant_date === null) {
                latest_relevant_date = task.COMPLETED;
            }
            if (task.COMPLETED > latest_relevant_date) {
                latest_relevant_date = task.COMPLETED;
            }
        }
        if (task.CREATED < earliest_relevant_date) {
            earliest_relevant_date = task.CREATED;
        }
    }

    return {start: earliest_relevant_date, end: latest_relevant_date}
}

export function get_dates_as_ms (earliest_relevant_date, latest_relevant_date) {
    let dates = []
    let day = Date.parse(earliest_relevant_date.split(" ")[0]);
    let latest_day = Date.parse(latest_relevant_date.split(" ")[0])
    while (day <= latest_day) {
        dates.push(day);
        day += 1000*60*60*24;
    }
    return dates
}

export function get_dates_as_strings(earliest_relevant_date, latest_relevant_date) {
    return get_dates_as_ms(earliest_relevant_date, latest_relevant_date).map(date => {
        let d = new Date(date);
        return d.getFullYear() + "-" + ("0"+(d.getMonth()+1)).slice(-2) + "-" + ("0" + d.getDate()).slice(-2);
    });
}

export function get_estimates_hours(tasks, dates) {
    let min_estimates_hours = [];
    let max_estimates_hours = [];
    for (const date of dates) {
        let min_estimate_sec = 0;
        let max_estimate_sec = 0;
        for (const task of tasks) {
            if (task.CREATED <= date) {
                if (task.COMPLETED <= date) {
                    continue;
                }
                min_estimate_sec += task.TIME_COST_ESTIMATES_SUMMARY.individual_work_min;
                max_estimate_sec += task.TIME_COST_ESTIMATES_SUMMARY.individual_work_max;
            }
        }
        min_estimates_hours.push(min_estimate_sec / 60 / 60);
        max_estimates_hours.push(max_estimate_sec / 60 / 60);
    }
    return {min: min_estimates_hours, max: max_estimates_hours}
}

export function estimated_time_remaining(element, tasks) {
    let date_range = get_date_range(tasks);
    let labels = get_dates_as_strings(date_range.start, date_range.end);
    let estimated_hours = get_estimates_hours(tasks, labels);
    new Chart(element, {
        type: 'line',
        labels: labels,
        data: {
            datasets: [
                {
                    label: "min estimated hours",
                    data: estimated_hours.min,
                },
                {
                    label: "max estimated hours",
                    data: estimated_hours.max,
                },
            ]
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
}
