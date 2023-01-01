export function mk_scales() {
    return {
        x: {
            type: 'category',
            min: 5,
            max: 50,
            grid: {
                color: 'rgba( 0, 0, 0, 0.1)'
            },
            title: {
                display: true,
                text: (ctx) => 'Date'
            }
        },
        y: {
            min: 0,
            max: 20,
            grid: {
                color: 'rgba( 0, 0, 0, 0.1)'
            },
            title: {
                display: true,
                text: (ctx) => 'Hours remainig'
            }
        }
    }
}
