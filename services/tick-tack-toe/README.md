Compilation 
--------------

```
poetry shell
./capnproto/c++/capnp compile -ocython level0.capnp
python setup_capnp.py build_ext --inplace
```
