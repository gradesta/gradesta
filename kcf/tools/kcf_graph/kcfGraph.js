export default {

    scales: {
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
    },

    get_date_range: (tasks) => {
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

        return (earliest_relevant_date, latest_relevant_date)
    }
}
