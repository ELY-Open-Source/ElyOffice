"""Objective checks for a rendered track, plus a spectrogram PNG."""
import argparse
import struct
import zlib

import numpy as np
from scipy import signal

from dsp import SR, butter, db, lufs, read_wav, short_term, true_peak

MARKS = (1.0, 4.0, 5.0, 12.0, 26.0, 41.4, 42.6, 44.0, 47.0, 50.0, 60.0, 68.0, 76.0, 78.0, 89.0)
COLORS = np.array([(0, 0, 4), (40, 11, 84), (101, 21, 110), (159, 42, 99), (212, 72, 66), (245, 125, 21),
                   (250, 193, 39), (252, 255, 164)], float)


def centroid(x):
    m = x.mean(1)
    spec = np.abs(np.fft.rfft(m * np.hanning(len(m))))
    f = np.fft.rfftfreq(len(m), 1 / SR)
    return float((f * spec).sum() / max(spec.sum(), 1e-12))


def band_share(x, lo, hi):
    m = x.mean(1)
    p = np.abs(np.fft.rfft(m * np.hanning(len(m)))) ** 2
    f = np.fft.rfftfreq(len(m), 1 / SR)
    return float(p[(f >= lo) & (f < hi)].sum() / max(p.sum(), 1e-20))


def seg(x, a, b):
    return x[int(a * SR):int(b * SR)]


def report(x):
    n = len(x)
    lines = [f"length {n} samples ({n / SR:.3f} s), channels {x.shape[1]}",
             f"integrated {lufs(x):.2f} LUFS, true peak {db(true_peak(x)):.2f} dBTP, sample peak {db(np.abs(x).max()):.2f} dBFS",
             f"DC offset L {x[:, 0].mean():.2e} R {x[:, 1].mean():.2e}"]
    t, st = short_term(x)
    lines.append(f"short-term max {st.max():.1f} LUFS at {t[st.argmax()] + 3:.1f} s")
    nz = np.nonzero(np.abs(x).max(1) > 0)[0]
    lines.append(f"first/last non-zero sample {nz[0] / SR:.4f} s / {nz[-1] / SR:.4f} s")
    lo = butter(x, "low", 120)
    corr = np.corrcoef(lo[:, 0], lo[:, 1])[0, 1]
    lines.append(f"L/R correlation below 120 Hz {corr:.4f}")
    d = np.abs(np.diff(x, axis=0)).max(1)
    lines.append(f"largest sample step {d.max():.3f} at {d.argmax() / SR:.3f} s")
    lines.append("bar  start  RMS dBFS  short-term LUFS  centroid Hz  1.5-4k share")
    for b in range(int(n / SR / 2)):
        s = seg(x, 2 * b, 2 * b + 2)
        rms = db(np.sqrt((s ** 2).mean()))
        stl = st[min(len(st) - 1, max(0, int((2 * b + 2 - 3) / 0.1)))]
        lines.append(f"{b:3d}  {2 * b:5.1f}  {rms:8.1f}  {stl:15.1f}  {centroid(s):11.0f}  {band_share(s, 1500, 4000):12.3f}")
    before, during, after = seg(x, 40.2, 41.3), seg(x, 41.6, 42.45), seg(x, 42.8, 43.9)
    lines.append("offline gag: centroid before {:.0f} Hz, during {:.0f} Hz, after {:.0f} Hz; >2 kHz share "
                 "{:.4f} / {:.4f} / {:.4f}; RMS {:.1f} / {:.1f} / {:.1f} dBFS".format(
                     centroid(before), centroid(during), centroid(after),
                     band_share(before, 2000, 24000), band_share(during, 2000, 24000), band_share(after, 2000, 24000),
                     db(np.sqrt((before ** 2).mean())), db(np.sqrt((during ** 2).mean())), db(np.sqrt((after ** 2).mean()))))
    return "\n".join(lines)


def colormap(v):
    v = np.clip(v, 0, 1) * (len(COLORS) - 1)
    i = np.minimum(v.astype(int), len(COLORS) - 2)
    f = (v - i)[..., None]
    return (COLORS[i] * (1 - f) + COLORS[i + 1] * f).astype(np.uint8)


def write_png(path, rgb):
    h, w, _ = rgb.shape
    raw = b"".join(b"\x00" + rgb[y].tobytes() for y in range(h))

    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(png)


def spectrogram(x, path, width=1800, height=560, strip=140):
    m = x.mean(1)
    seconds = len(m) / SR
    hop = len(m) / width
    nfft = 4096
    f, _, z = signal.stft(m, SR, nperseg=nfft, noverlap=nfft - int(hop), boundary=None, padded=True)
    mag = 20 * np.log10(np.abs(z) + 1e-9)
    cols = np.linspace(0, mag.shape[1] - 1, width).astype(int)
    rows = np.geomspace(30, 20000, height)[::-1]
    idx = np.clip(np.searchsorted(f, rows), 0, len(f) - 1)
    img = mag[idx][:, cols]
    top = img.max()
    rgb = colormap((img - (top - 90)) / 90)
    for fr in (100, 1000, 10000, 1500, 4000):
        r = np.argmin(np.abs(rows - fr))
        rgb[r, ::4] = (90, 90, 90) if fr in (1500, 4000) else (150, 150, 150)
    band = np.zeros((strip, width, 3), np.uint8) + 18
    t = np.arange(len(m)) / SR
    for c in range(width):
        s = m[int(c * hop):int((c + 1) * hop)]
        if len(s):
            r = db(np.sqrt((s ** 2).mean()))
            h = int(np.clip((r + 50) / 50, 0, 1) * (strip - 4))
            band[strip - 2 - h:strip - 2, c] = (120, 200, 255)
    for mark in MARKS:
        c = int(mark / seconds * width)
        if 0 <= c < width:
            rgb[::3, c] = (255, 255, 255)
            band[::2, c] = (255, 255, 255)
    for sec in range(0, int(seconds) + 1, 2):
        c = min(width - 1, int(sec / seconds * width))
        band[:8 if sec % 10 else 16, c] = (200, 200, 200)
    write_png(path, np.concatenate([band, rgb], 0))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("wav")
    ap.add_argument("--png")
    a = ap.parse_args()
    x = read_wav(a.wav)
    print(report(x))
    if a.png:
        spectrogram(x, a.png)
        print(f"wrote {a.png}")


if __name__ == "__main__":
    main()
