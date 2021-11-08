#!/bin/sh
capnp compile -oc++ level0.capnp
capnp compile -ocython level0.capnp
python setup_capnp.py build_ext --inplace
