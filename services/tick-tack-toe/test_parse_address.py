import parse_address

def test_parse_address():
    test_addresses = {
        "portAddress": "gradesta://example.com/en/tick-tack-toe/fool/baz?arg=val",
        "portAddressNoQargs": "gradesta://example.com/en/tick-tack-toe/fool/baz",
        "portAddressNoPath": "gradesta://example.com/en/tick-tack-toe/",
        "portAddressQarg": "gradesta://example.com/en/tick-tack-toe/?arg=val",
        "portAddressQargs": "gradesta://example.com/en/tick-tack-toe/?arg=val&foo=bar&baz=baf",
        "portAddressWithPort": "gradesta://example.com:435/en/tick-tack-toe/fool/baz?arg=val",
        "portAddressWithPortNoQargs": "gradesta://example.com:435/en/tick-tack-toe/fool/baz",
        "unixSocketAddress": "/var/gradesta/services:/en/tick-tack-toe/fool/baz?arg=val",
        "unixSocketAddressNoQargs": "/var/gradesta/services:/en/tick-tack-toe/fool/baz",
    }
    for id, address in test_addresses.items():
        capnp_addr = parse_address.parse_address(address)
        assert address == parse_address.to_string(capnp_addr)
