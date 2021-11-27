# https://stackoverflow.com/a/3707022/2126889
def pytest_collect_file(path, parent):
    if path.ext == ".py":
        try:
            return parent.Module(path, parent)
        except:
            pass
