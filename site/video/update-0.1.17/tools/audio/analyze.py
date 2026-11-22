"""Loudness, peaks and a spectrogram PNG for a rendered track."""
import argparse
import sys
from pathlib import Path

import numpy as np
from scipy import signal

sys.path.insert(0, str(Path(__file__).resolve().parent))

from dsp import SR, butter, db, lufs, read_wav, short_term, true_peak  # noqa: E402
from grid import CHAPTERS, HITS  # noqa: E402


def centroid(x):
    m = x.mean(1)
    spec = np.abs(np.fft.rfft(m * np.hanning(len(m))))
    f = np.fft.rfftfreq(len(m), 1 / SR)
    return float((f * spec).sum() / max(spec.sum(), 1e-12))


BANDS = ((20, 120), (120, 500), (500, 2000), (2000, 6000), (6000, 20000))


def bands(x):
    """Power share per band in dB, sub to air."""
    m = x.mean(1)
    p = np.abs(np.fft.rfft(m)) ** 2
    f = np.fft.rfftfreq(len(m), 1 / SR)
    total = p.sum() + 1e-20
    return " ".join(f"{10 * np.log10(p[(f >= a) & (f < b)].sum() / total + 1e-12):6.1f}" for a, b in BANDS)


def report(x):
    n = len(x)
    t, st = short_term(x)
    nz = np.nonzero(np.abs(x).max(1) > 1e-6)[0]
    lo = butter(x, "low", 120)
    d = np.abs(np.diff(x, axis=0)).max(1)
    lines = [f"length {n} samples ({n / SR:.3f} s), channels {x.shape[1]}",
             f"integrated {lufs(x):.2f} LUFS, true peak {db(true_peak(x)):.2f} dBTP, "
             f"sample peak {db(np.abs(x).max()):.2f} dBFS",
             f"short-term max {st.max():.1f} LUFS at {t[st.argmax()] + 3:.1f} s, "
             f"loudness range {np.percentile(st[st > -70], 95) - np.percentile(st[st > -70], 10):.1f} LU",
             f"DC L {x[:, 0].mean():.1e} R {x[:, 1].mean():.1e}; "
             f"L/R correlation below 120 Hz {np.corrcoef(lo[:, 0], lo[:, 1])[0, 1]:.3f}",
             f"first/last sample above -120 dBFS {nz[0] / SR:.3f} s / {nz[-1] / SR:.3f} s" if len(nz) else "silent",
             f"largest sample step {d.max():.3f} at {d.argmax() / SR:.3f} s",
             "chapter  from    to  RMS dBFS  peak dBFS  LUFS  centroid Hz   dB <120  -500  -2k   -6k  >6k"]
    for a, b in zip(CHAPTERS, CHAPTERS[1:]):
        s = x[int(a * SR):int(b * SR)]
        rms, pk = db(np.sqrt((s ** 2).mean())), db(np.abs(s).max())
        lines.append(f"         {a:5.1f} {b:5.1f}  {rms:8.1f}  {pk:9.1f}  {lufs(s):5.1f}  {centroid(s):11.0f}"
                     f"  {bands(s)}")
    return "\n".join(lines)


def spectrogram(x, path, start=0.0, end=None, title=""):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    end = end or len(x) / SR
    m = x[int(start * SR):int(end * SR)].mean(1)
    nfft = 4096 if end - start > 20 else 2048
    f, tt, z = signal.stft(m, SR, nperseg=nfft, noverlap=nfft * 7 // 8)
    mag = 20 * np.log10(np.abs(z) + 1e-9)
    fig, (a1, a2) = plt.subplots(2, 1, figsize=(18, 7.5), gridspec_kw={"height_ratios": [1, 4]}, sharex=True)
    hop = int(SR * 0.01)
    env = [db(np.sqrt((m[i:i + hop] ** 2).mean() + 1e-20)) for i in range(0, len(m) - hop, hop)]
    a1.plot(start + np.arange(len(env)) * 0.01, env, lw=0.6, color="#1c1f26")
    a1.set_ylim(-70, 0)
    a1.set_ylabel("dBFS")
    top = mag.max()
    a2.pcolormesh(start + tt, f, mag, vmin=top - 90, vmax=top, cmap="magma", shading="auto")
    a2.set_yscale("log")
    a2.set_ylim(30, 20000)
    a2.set_ylabel("Hz")
    a2.set_xlabel("film seconds")
    for h in HITS:
        if start <= h <= end:
            a1.axvline(h, color="#3b6fe0", lw=0.5, alpha=0.6)
            a2.axvline(h, color="white", lw=0.4, alpha=0.35)
    a1.set_title(title)
    fig.tight_layout()
    fig.savefig(path, dpi=80)
    plt.close(fig)


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("wav")
    ap.add_argument("--png", help="spectrogram path")
    ap.add_argument("--from", dest="start", type=float, default=0.0)
    ap.add_argument("--to", dest="end", type=float)
    a = ap.parse_args()
    x = read_wav(a.wav)
    print(report(x))
    if a.png:
        spectrogram(x, a.png, a.start, a.end, Path(a.wav).name)
        print(f"wrote {a.png}")


if __name__ == "__main__":
    main()
