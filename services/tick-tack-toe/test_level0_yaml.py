import pytest
import io
import tempfile

import level0_yaml


def test_data_update_to_capnp_and_back():
    fd2 = tempfile.TemporaryFile()
    with open("./example_yaml/hello_world_data_update.yml", "rb") as fd1:
        level0_yaml.to_capnp(fd1, fd2)
        fd3 = io.BytesIO()
        fd2.seek(0)
        level0_yaml.to_yaml(fd2, fd3)
        fd3.seek(0)
        fd3.name = "foo.yml"
        fd1.seek(0)
        assert level0_yaml.compare(fd1, fd3) == {}
