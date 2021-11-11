from setuptools import setup, find_packages

setup(
    name='gradesta-tick-tack-toe',
    version='0.0.1',
    description='Gradesta tick-tack-toe service',
    author='Timothy Hobbs',
    author_email='goodnight@gradesta.com',
    url='https://gradesta.org',
    packages=find_packages(include=['tick_tack_toe', 'tick_tack_toe.*']),
    install_requires=[
        'gradesta',
    ],
    setup_requires=['pytest-runner', 'black'],
    tests_require=['pytest'],
    entry_points={
        'console_scripts': ['gradesta_tick_tack_toe=tick_tack_toe.tick_tack_toe:main']
    },
    classifiers=['License :: OSI Approved ::  GNU Affero General Public License v3 or later (AGPLv3+)'],
)
