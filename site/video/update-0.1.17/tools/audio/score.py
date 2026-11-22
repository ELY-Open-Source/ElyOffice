"""The score: 120 BPM, F major, G from bar 29."""
import harmony
import melody
import rhythm
from dsp import reseed
from grid import Mix, SEED
from master import master, stem_report


def build_music():
    reseed(SEED)
    mix = Mix()
    melody.touch(mix)
    harmony.piano_part(mix)
    harmony.pads(mix)
    harmony.sub(mix)
    harmony.harp(mix)
    harmony.pizz(mix)
    rhythm.marimba(mix)
    rhythm.drums(mix)
    rhythm.transitions(mix)
    rhythm.hits(mix)
    melody.tunes(mix)
    return master(mix), stem_report(mix)
