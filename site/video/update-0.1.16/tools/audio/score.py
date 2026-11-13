"""The score: 120 BPM, 45 bars, D major, then E."""
import band
import instruments as ins
import melody
from dsp import reseed, SR
from grid import Mix, SEED
from master import master, stem_report


def breath(mix):
    """0-2 s: one reversed tine drawing in toward the strike."""
    y = ins.kalimba(86, 0.5, 0.9)[::-1]
    mix.add("fx", y, 2.0 - len(y) / SR, 0.35, hall=0.4)


def build_music():
    reseed(SEED)
    mix = Mix()
    breath(mix)
    band.drums(mix)
    band.bass(mix)
    band.guitar_part(mix)
    band.reed(mix)
    band.hits(mix)
    band.render_kicks(mix)
    melody.kalimba(mix)
    melody.whistle(mix)
    melody.glock(mix)
    return master(mix), stem_report(mix)
