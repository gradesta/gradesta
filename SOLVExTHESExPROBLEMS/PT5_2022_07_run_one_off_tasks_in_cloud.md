Public tender #5: I need to run one off tasks in docker on the cloud so easilly I don't even need to think about it.

Keywords: docker, cloud, celery, spawn

Needs:
-------

- Simple
- Simple
- High performance instances
- Simple
- In docker

Max price: €2 per hour on a very very powerfull machine

What do I want?
---------------

```
cat my_video_file | cloud_docker_runner --cpus=128 --mem=20gb --timeout=2h --secrets="$HOME/.config/s3credentials" ffmpegTranscodeAndUploadDockerimage
```

This would pull some credentials from a json config file in the user's home directory and bill the account associated with those credntials for the time that the container was running.

Reads from `stdin`. Once `stdin` was consumed the command would exit.

It would be possible to run commands like:

```
cloud_docker_runner ps
```

And

```
cloud_docker_runner exec <container_name> -- bash
```

What is available?
------------------

EC2 spot instances? Well they are too complicated to set up, need a simple single command solution. Really not sure what to do.
