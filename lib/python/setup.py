from setuptools import setup, find_packages

setup(
    name='gradesta',
    version='0.0.1',
    description='Gradesta libraries for interacting using the gradesta protocol using python',
    author='Timothy Hobbs',
    author_email='goodnight@gradesta.com',
    url='https://gradesta.org',
    install_requires=[
        "gradesta",
        "PyYAML",
        "pycapnp",
        "numpy",
        "Jinja2",
        "Cython",
        "deepdiff",
        "zmq",
    ],
    setup_requires=['pytest-runner', 'black'],
    tests_require=['pytest'],
    classifiers=['License :: OSI Approved ::  GNU Lesser General Public License v3 or later (LGPLv3+)'],
)
