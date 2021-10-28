const path = require('path');
const CompressionPlugin = require('compression-webpack-plugin');

module.exports = {
    plugins: [new CompressionPlugin()],
    resolve: {
        extensions: ["", ".webpack.js", ".web.js", ".js", ".ts"]
    },
    entry: {
        greg: './ts/greg.ts',
        ticktacktoe: './examples/ts/ticktacktoe.ts',
        ticktacktoecell: './examples/ts/ticktacktoecell.ts',
    },
    module: {
        rules: [
            {
                test: /\.css$/i,
                use: ['style-loader', 'css-loader'],
            },
            {
                test: /\.(png|jpe?g|gif|svg)$/i,
                loader: 'file-loader',
            },
            {
                test: /\.less$/i,
                use: ['style-loader', 'css-loader', 'less-loader'],
            }
        ],
    },
    resolve: {
        extensions: ['.ts', '.tsx', '.js'],
    },
    output: {
        path: path.resolve(__dirname, 'build/js/dist')
    },
    mode: "development",
    watchOptions: {
        ignored: '*/node_modules',
        ignored: 'node_modules',
        ignored: ".*",
    }
}
