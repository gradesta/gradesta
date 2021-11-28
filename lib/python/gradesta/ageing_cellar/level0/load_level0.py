import pathlib
import capnp
# https://github.com/capnproto/pycapnp/issues/242
#import level0_capnp as level0
level0_capnp = str(pathlib.Path(__file__).parent.parent.parent.parent.parent.parent.resolve() / "protocol" / "level0.capnp")
level0 = capnp.load(level0_capnp)
