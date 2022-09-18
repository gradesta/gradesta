Public tender #6: I want a universal/cross platform CI/CD config format

Keywords: docker, cloud, CI/CD, continuous integration, continuous deployment

Every time I move to a new CI/CD platform I have to learn a new YAML syntax for configuring the pipelines. There should be a universal format that "compiles" to all the big player's "proprietery" formats so that I would have "universal-ci.yaml" → ".circleci/config.yaml" ect. based on which platform I wanted to use. Should support:

- Gitlab CI/CD
- Github CI/CD
- Circle CI
- Travis CI

