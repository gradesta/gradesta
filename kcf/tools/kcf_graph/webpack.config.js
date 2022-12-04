const path = require('path')

module.exports = {
  entry: './src/kcf-graph.js',
  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'kcf-graph.js',
    library: 'kcfGraph'
  }
}
