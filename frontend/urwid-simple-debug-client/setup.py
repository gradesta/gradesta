from setuptools import setup, find_packages

setup(
    name='gradesta-urwid-debug-client',
    version='0.0.1',
    description='Gradesta urwid debug client',
    author='Timothy Hobbs',
    author_email='goodnight@gradesta.com',
    url='https://gradesta.org',
    packages=find_packages(include=['gradesta_urwid_debug_client', 'gradesta_urwid_debug_client.*']),
    install_requires=[
        'urwid',
        'gradesta'
    ],
    setup_requires=['pytest-runner', 'black'],
    tests_require=['pytest'],
    entry_points={
        'console_scripts': ['gradesta_urwid_debug_client=gradesta_urwid_debug_client.client:main']
    },
    classifiers=['License :: OSI Approved ::  GNU Affero General Public License v3 or later (AGPLv3+)'],
)
