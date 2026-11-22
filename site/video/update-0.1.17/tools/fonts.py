"""Saves one face of a font collection alone."""
import os
import sys

from fontTools.ttLib import TTCollection

source, index, target = sys.argv[1], int(sys.argv[2]), sys.argv[3]
os.makedirs(os.path.dirname(target), exist_ok=True)
partial = f"{target}.{os.getpid()}"
TTCollection(source).fonts[index].save(partial)
os.replace(partial, target)
