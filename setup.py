#!/usr/bin/python

from opcodes import __version__, __author__, __email__
from distutils.core import setup


def read_text_file(path):
    import os
    with open(os.path.join(os.path.dirname(__file__), path)) as f:
        return f.read()


setup(
    name="opcodes",
    version=__version__,
    description="Database of Processor Instructions/Opcodes",
    long_description=read_text_file("readme.rst"),
    author=__author__,
    author_email=__email__,
    url="https://github.com/Maratyszcza/Opcodes",
    packages=["opcodes"],
    package_data={"opcodes": ["x86.xml", "x86_64.xml"]},
    keywords=["assembly", "assembler", "asm"],
    requires=[],
    classifiers=[
        "Development Status :: 3 - Alpha",
        "Intended Audience :: Developers",
        "Intended Audience :: Science/Research",
        "License :: OSI Approved :: BSD License",
        "Operating System :: OS Independent",
        "Programming Language :: Assembly",
        "Topic :: Scientific/Engineering",
        "Topic :: Software Development",
        "Topic :: Software Development :: Assemblers",
        "Topic :: Software Development :: Documentation"
    ])
