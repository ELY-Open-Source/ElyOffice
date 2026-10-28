"""Shared DSP: filters, oscillators, envelopes, space, dynamics and I/O."""
import wave
from pathlib import Path

import numpy as np
from scipy import signal
from scipy.ndimage import minimum_filter1d, uniform_filter1d

SR = 48000
BEAT = 0.5
BAR = 2.0
LENGTH = 60.0
N = int(LENGTH * SR)

_rng = np.random.default_rng(0)


def reseed(seed):
    global _rng
    _rng = np.random.default_rng(seed)


def rng():
    return _rng


def hz(m):
    return 440.0 * 2 ** ((np.asarray(m, float) - 69) / 12)


def secs(n):
    return np.arange(n) / SR


def ns(t):
    return int(round(t * SR))


def noise(n, ch=None):
    return _rng.standard_normal(n if ch is None else (n, ch))


# Filters ---------------------------------------------------------------

def biquad(kind, f, q=0.7071, gain_db=0.0):
    """RBJ cookbook biquad as a one-section SOS array."""
    f = float(np.clip(f, 5.0, 0.46 * SR))
    w = 2 * np.pi * f / SR
    c, s = np.cos(w), np.sin(w)
    alpha = s / (2 * q)
    amp = 10 ** (gain_db / 40)
    if kind == "low":
        b, a = [(1 - c) / 2, 1 - c, (1 - c) / 2], [1 + alpha, -2 * c, 1 - alpha]
    elif kind == "high":
        b, a = [(1 + c) / 2, -(1 + c), (1 + c) / 2], [1 + alpha, -2 * c, 1 - alpha]
    elif kind == "band":
        b, a = [alpha, 0.0, -alpha], [1 + alpha, -2 * c, 1 - alpha]
    elif kind == "peak":
        b, a = [1 + alpha * amp, -2 * c, 1 - alpha * amp], [1 + alpha / amp, -2 * c, 1 - alpha / amp]
    elif kind in ("lowshelf", "highshelf"):
        k = 2 * np.sqrt(amp) * alpha
        sign = 1 if kind == "lowshelf" else -1
        b = [amp * ((amp + 1) - sign * (amp - 1) * c + k),
             sign * 2 * amp * ((amp - 1) - sign * (amp + 1) * c),
             amp * ((amp + 1) - sign * (amp - 1) * c - k)]
        a = [(amp + 1) + sign * (amp - 1) * c + k,
             -sign * 2 * ((amp - 1) + sign * (amp + 1) * c),
             (amp + 1) + sign * (amp - 1) * c - k]
    else:
        raise ValueError(f"unknown filter kind {kind!r}")
    b, a = np.array(b) / a[0], np.array(a) / a[0]
    return np.array([[b[0], b[1], b[2], 1.0, a[1], a[2]]])


def eq(x, kind, f, q=0.7071, gain_db=0.0):
    return signal.sosfilt(biquad(kind, f, q, gain_db), x, axis=0)


def butter(x, kind, f, order=2):
    kind = {"low": "lowpass", "high": "highpass", "band": "bandpass"}[kind]
    if np.ndim(f):
        f = [min(v, 0.46 * SR) for v in f]
    else:
        f = min(f, 0.46 * SR)
    return signal.sosfilt(signal.butter(order, f, kind, fs=SR, output="sos"), x, axis=0)


def tv(x, kind, freq, q=0.7071, block=64):
    """Biquad whose corner follows freq (scalar or per-sample array); state carries across blocks."""
    freq = np.broadcast_to(np.asarray(freq, float), (len(x),))
    y = np.empty_like(x)
    zi = np.zeros((1, 2) + x.shape[1:])
    for s in range(0, len(x), block):
        e = min(s + block, len(x))
        y[s:e], zi = signal.sosfilt(biquad(kind, freq[(s + e) // 2], q), x[s:e], axis=0, zi=zi)
    return y


# Oscillators -----------------------------------------------------------

def phase(freq, n=None, start=None):
    freq = np.asarray(freq, float)
    if freq.ndim == 0:
        freq = np.full(n, float(freq))
    p0 = _rng.uniform() if start is None else start
    return (p0 + (np.cumsum(freq) - freq[0]) / SR) % 1.0, freq / SR


def _blep(p, dt):
    r = np.zeros_like(p)
    lo = p < dt
    x = p[lo] / dt[lo]
    r[lo] = 2 * x - x * x - 1
    hi = p > 1 - dt
    x = (p[hi] - 1) / dt[hi]
    r[hi] = x * x + 2 * x + 1
    return r


def saw(freq, n=None, start=None):
    p, dt = phase(freq, n, start)
    return 2 * p - 1 - _blep(p, dt)


def pulse(freq, width=0.5, n=None, start=None):
    p, dt = phase(freq, n, start)
    q = (p + width) % 1.0
    return (2 * p - 1 - _blep(p, dt)) - (2 * q - 1 - _blep(q, dt))


def sine(freq, n=None, start=0.0):
    p, _ = phase(freq, n, start)
    return np.sin(2 * np.pi * p)


# Envelopes and placement -----------------------------------------------

def env(n, attack=0.005, release=0.05, hold=None, decay=None, sustain=1.0):
    """Click-free envelope: sine-squared attack, optional decay, cosine-squared release ending at zero."""
    t = secs(n)
    e = np.sin(np.clip(t / max(attack, 1e-4), 0, 1) * np.pi / 2) ** 2
    if decay:
        e = e * (sustain + (1 - sustain) * np.exp(-t / decay))
    if hold is None:
        hold = n / SR - release
    r = np.clip((t - hold) / max(release, 1e-4), 0, 1)
    return e * np.cos(r * np.pi / 2) ** 2


def edges(x, a=0.002, r=0.01):
    x = np.array(x, float)
    ai, ri = max(2, ns(a)), max(2, ns(r))
    up, down = np.sin(np.linspace(0, 1, ai) * np.pi / 2) ** 2, np.cos(np.linspace(0, 1, ri) * np.pi / 2) ** 2
    if x.ndim == 2:
        up, down = up[:, None], down[:, None]
    x[:ai] *= up
    x[-ri:] *= down
    return x


def pan(x, p=0.0):
    th = (np.clip(p, -1, 1) + 1) * np.pi / 4
    return np.stack([x * np.cos(th), x * np.sin(th)], 1) * np.sqrt(2)


def width(x, amount, mono_below=150.0):
    mid, side = (x[:, 0] + x[:, 1]) / 2, (x[:, 0] - x[:, 1]) / 2 * amount
    side = butter(side, "high", mono_below, 2)
    return np.stack([mid + side, mid - side], 1)


# Space -------------------------------------------------------------------

def impulse(seconds, t60, seed, predelay=0.012, damp=8000.0, build=0.015):
    """Stereo reverb impulse: decorrelated noise with low, mid and high decay times plus early reflections."""
    g = np.random.default_rng(seed)
    n = ns(seconds)
    t = secs(n)
    ir = np.zeros((n, 2))
    for ch in range(2):
        w = g.standard_normal(n)
        lo, hi = butter(w, "low", 350), butter(w, "high", 3000)
        ir[:, ch] = sum(b * 10 ** (-3 * t / T) for b, T in zip((lo, w - lo - hi, hi), t60))
    ir = butter(ir, "low", damp) * (np.minimum(1, t / build) ** 1.5)[:, None]
    scale = np.sqrt((ir ** 2).mean())
    for ch in range(2):
        for _ in range(9):
            d = int(g.uniform(0.003, 0.06) * SR)
            ir[d, ch] += g.choice([-1, 1]) * g.uniform(8, 20) * scale * np.exp(-d / SR / 0.04)
    ir = edges(ir, 0.0005, seconds * 0.2)
    ir = np.concatenate([np.zeros((ns(predelay), 2)), ir])
    return ir / np.sqrt((ir ** 2).sum(0))


def convolve(x, ir):
    return np.stack([signal.fftconvolve(x[:, c], ir[:, c])[:len(x)] for c in range(2)], 1)


def pingpong(x, delay, fb=0.42, taps=7, lp=4200.0, hp=320.0):
    d, n = ns(delay), len(x)
    y, out = x.mean(1), np.zeros((n, 2))
    for k in range(1, taps + 1):
        y = butter(butter(y, "low", lp, 1), "high", hp, 1)
        if k * d >= n:
            break
        out[k * d:, (k - 1) % 2] += y[:n - k * d] * fb ** (k - 1)
    return out


def chorus(x, rate=0.23, depth=0.0025, centre=0.011, mix=0.45):
    n = len(x)
    t = secs(n)
    out = np.empty_like(x)
    for ch, ph in ((0, 0.0), (1, np.pi / 2)):
        pos = np.arange(n) - (centre + depth * np.sin(2 * np.pi * rate * t + ph)) * SR
        i0 = np.floor(pos).astype(int)
        fr = pos - i0
        wet = x[np.clip(i0, 0, n - 1), ch] * (1 - fr) + x[np.clip(i0 + 1, 0, n - 1), ch] * fr
        wet[pos < 0] = 0
        out[:, ch] = x[:, ch] + mix * wet
    return out / (1 + mix)


# Dynamics ----------------------------------------------------------------

def compress(x, threshold_db, ratio, attack, release, knee=6.0, block=48):
    """Feed-forward peak compressor; the detector runs on blocks, the gain is interpolated per sample."""
    lvl = np.abs(x).max(1) if x.ndim == 2 else np.abs(x)
    nb = -(-len(lvl) // block)
    peaks = np.pad(lvl, (0, nb * block - len(lvl))).reshape(nb, block).max(1)
    over = 20 * np.log10(peaks + 1e-12) - threshold_db
    slope = 1 - 1 / ratio
    gr = np.where(over <= -knee / 2, 0.0,
                  np.where(over >= knee / 2, over * slope, slope * (over + knee / 2) ** 2 / (2 * knee)))
    att, rel = np.exp(-block / (attack * SR)), np.exp(-block / (release * SR))
    sm, acc = np.empty(nb), 0.0
    for i, v in enumerate(gr):
        acc = v + (acc - v) * (att if v > acc else rel)
        sm[i] = acc
    centres = (np.arange(nb) + 0.5) * block
    g = 10 ** (-np.interp(np.arange(len(lvl)), centres, sm) / 20)
    return x * (g[:, None] if x.ndim == 2 else g)


def true_peak(x):
    return np.abs(signal.resample_poly(x, 4, 1, axis=0)).max()


def db(v):
    return 20 * np.log10(max(float(v), 1e-12))


def limiter(x, ceiling_db, release=0.08, look=96, block=32):
    """Look-ahead brickwall on 4x-oversampled peaks. The smoothed gain never exceeds what any nearby peak needs."""
    c = 10 ** (ceiling_db / 20)
    n = len(x)
    up = np.abs(signal.resample_poly(x, 4, 1, axis=0)).max(1)[:n * 4]
    pk = np.maximum(np.pad(up, (0, n * 4 - len(up))).reshape(n, 4).max(1), np.abs(x).max(1))
    g = minimum_filter1d(np.minimum(1.0, c / np.maximum(pk, 1e-12)), 2 * look + 1, mode="nearest")
    nb = -(-n // block)
    gb = np.pad(g, (0, nb * block - n), constant_values=1.0).reshape(nb, block).min(1)
    a, acc, out = np.exp(-block / (release * SR)), 1.0, np.empty(nb)
    for i, v in enumerate(gb):
        acc = v if v < acc else v - (v - acc) * a
        out[i] = acc
    smooth = uniform_filter1d(np.repeat(out, block)[:n], 2 * block + 1, mode="nearest")
    return x * smooth[:, None]


_K1 = ([1.53512485958697, -2.69169618940638, 1.19839281085285], [1.0, -1.69065929318241, 0.73248077421585])
_K2 = ([1.0, -2.0, 1.0], [1.0, -1.99004745483398, 0.99007225036621])


def _kweighted_blocks(x, seconds, hop):
    x = np.atleast_2d(np.asarray(x, float).T).T
    y = signal.lfilter(*_K2, signal.lfilter(*_K1, x, axis=0), axis=0)
    blk, step = ns(seconds), ns(hop)
    cs = np.concatenate([np.zeros((1, y.shape[1])), np.cumsum(y ** 2, 0)])
    starts = np.arange(0, len(y) - blk + 1, step)
    return starts, ((cs[starts + blk] - cs[starts]) / blk).sum(1)


def lufs(x):
    """ITU-R BS.1770-4 integrated loudness (48 kHz)."""
    _, z = _kweighted_blocks(x, 0.4, 0.1)
    loud = -0.691 + 10 * np.log10(z + 1e-20)
    keep = loud > -70
    if not keep.any():
        return -np.inf
    rel = -0.691 + 10 * np.log10(z[keep].mean()) - 10
    keep &= loud > rel
    return -0.691 + 10 * np.log10(z[keep].mean())


def short_term(x, hop=0.1):
    starts, z = _kweighted_blocks(x, 3.0, hop)
    return starts / SR, -0.691 + 10 * np.log10(z + 1e-20)


# I/O ---------------------------------------------------------------------

def write_wav(path, x):
    x = np.atleast_2d(np.asarray(x, float).T).T
    q = np.ascontiguousarray(np.round(np.clip(x, -1, 1 - 2 ** -23) * 2 ** 23).astype("<i4"))
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    with wave.open(str(path), "wb") as w:
        w.setnchannels(x.shape[1])
        w.setsampwidth(3)
        w.setframerate(SR)
        w.writeframes(q.view(np.uint8).reshape(-1, 4)[:, :3].tobytes())


def read_wav(path):
    """Read a 16- or 24-bit PCM WAV as float (frames, channels)."""
    with wave.open(str(path), "rb") as w:
        ch, width, rate, frames = w.getnchannels(), w.getsampwidth(), w.getframerate(), w.getnframes()
        raw = w.readframes(frames)
    if rate != SR:
        raise ValueError(f"{path}: {rate} Hz, expected {SR}")
    if width == 3:
        b = np.frombuffer(raw, np.uint8).reshape(-1, 3)
        v = (b[:, 0].astype(np.int32) | (b[:, 1].astype(np.int32) << 8) | (b[:, 2].astype(np.int32) << 16))
        v = np.where(v >= 1 << 23, v - (1 << 24), v) / 2 ** 23
    elif width == 2:
        v = np.frombuffer(raw, "<i2") / 2 ** 15
    else:
        raise ValueError(f"{path}: {8 * width}-bit PCM is not supported")
    return v.reshape(-1, ch).astype(float)
