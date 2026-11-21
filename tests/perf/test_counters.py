"""The macOS and Windows paging counters (R20.58)."""

import datetime
import os
import subprocess
import sys
import tempfile
import time
import unittest
import unittest.mock

import counters
import judge

VM_STAT = """Mach Virtual Memory Statistics: (page size of 16384 bytes)
Pages free:                                5000.
Pageins:                              107906298.
Pageouts:                                108263.
Swapins:                                 491950.
Swapouts:                               1133006.
"""


def reading(swapped_in, swapped_out, pageouts):
    return {"in": swapped_in, "out": swapped_out, "logged": {"pageouts KiB": pageouts}}


def until(sampler, n):
    """Polls for `n` samples, at most 10 s."""
    deadline = time.monotonic() + 10
    while len(sampler.samples) < n and time.monotonic() < deadline and sampler.alive():
        time.sleep(0.05)
    return sampler.samples


class Counters(unittest.TestCase):
    def test_vm_stat_reads_swap_in_kib_and_keeps_pageouts_apart(self):
        done = subprocess.CompletedProcess(["vm_stat"], 0, stdout=VM_STAT.encode())
        with unittest.mock.patch("subprocess.run", return_value=done):
            read = counters.mac()
        self.assertEqual(read, {"in": 491950 * 16, "out": 1133006 * 16, "logged": {"pageouts KiB": 108263 * 16}})
        short = subprocess.CompletedProcess(["vm_stat"], 0, stdout=VM_STAT.replace("Swapouts", "Swapped").encode())
        with unittest.mock.patch("subprocess.run", return_value=short), self.assertRaisesRegex(RuntimeError, "lacks"):
            counters.mac()

    def test_a_sample_is_the_change_over_its_second(self):
        si, so, logged = counters.rates(reading(100, 50, 7), reading(300, 50, 9), 2.0)
        self.assertEqual((si, so, logged), (100.0, 0, {"pageouts KiB": 2}))

    def test_an_unmeasured_swap_in_stays_unmeasured(self):
        windows = {"in": None, "out": 8, "logged": {"page reads": 3}}
        si, so, logged = counters.rates(windows, {**windows, "out": 12, "logged": {"page reads": 9}}, 1.0)
        self.assertEqual((si, so, logged), (None, 4, {"page reads": 6}))

    def test_a_swap_out_lands_in_its_own_second_and_pageouts_stay_out(self):
        values = iter([reading(0, 0, 0), reading(0, 0, 500), reading(0, 64, 500), reading(0, 64, 500)])
        with tempfile.TemporaryDirectory() as work:
            sampler = counters.Sampler(os.path.join(work, "paging.log"), lambda: next(values))
            taken = until(sampler, 3)
            sampler.stop()
            with open(sampler.path, encoding="utf-8") as trace:
                lines = trace.read().splitlines()
        self.assertEqual([so for _, _, so in taken[:3]], [0, 64, 0])
        self.assertTrue(all(len(sample) == 3 for sample in taken), "pageouts are not judged")
        self.assertIn("pageouts KiB=500", lines[0])
        self.assertLess(taken[0][0], taken[1][0])
        self.assertEqual(taken[0][0].utcoffset().total_seconds(), 0, "stamped in UTC")

    def test_a_rising_pages_output_is_a_swap_out_and_swap_in_stays_unmeasured(self):
        def windows(out):
            return {"in": None, "out": out, "logged": {"page reads": 0, "pages input": 0}}

        values = iter([windows(151004), windows(151004), windows(151068), windows(151068)])
        with tempfile.TemporaryDirectory() as work:
            sampler = counters.Sampler(os.path.join(work, "paging.log"), lambda: next(values))
            taken = until(sampler, 3)
            sampler.stop()
        self.assertEqual([(si, so) for _, si, so in taken[:3]], [(None, 0), (None, 64), (None, 0)])

    def test_pdh_refuses_invalid_data_the_call_called_a_success(self):
        import ctypes

        def reader(status):
            class Fake:
                def PdhCollectQueryData(self, query):
                    return 0

                def PdhGetRawCounterValue(self, handle, kind, raw):
                    raw._obj.status, raw._obj.first = status, 1234
                    return 0

            class Raw(ctypes.Structure):
                _fields_ = [("status", ctypes.c_ulong), ("first", ctypes.c_longlong)]

            pdh = counters.Pdh.__new__(counters.Pdh)
            pdh.ctypes, pdh.raw, pdh.page, pdh.pdh, pdh.query = ctypes, Raw, 4096, Fake(), None
            pdh.counters = {key: None for key in counters.Pdh.NAMES}
            return pdh.read

        for valid in counters.Pdh.VALID:
            self.assertEqual(reader(valid)()["out"], 1234 * 4, f"status {valid} is data")
        # PDH_CSTATUS_INVALID_DATA, with the call itself a success.
        with self.assertRaisesRegex(RuntimeError, "invalid data.*0xC0000BBA"):
            reader(0xC0000BBA)()

    def test_a_reader_that_fails_stops_the_run(self):
        def broken():
            raise OSError("no counters")

        with tempfile.TemporaryDirectory() as work:
            sampler = counters.Sampler(os.path.join(work, "paging.log"), broken)
            sampler.thread.join(5)
            with self.assertRaisesRegex(RuntimeError, "no counters"):
                sampler.alive()

    def test_counters_that_start_but_never_sample_are_stopped(self):
        made = []
        class Seen(counters.Sampler):
            def __init__(self, *a):
                super().__init__(*a)
                made.append(self)
        def broken():
            raise OSError("no counters here")
        with tempfile.TemporaryDirectory() as work, \
             unittest.mock.patch("counters.reader", lambda: broken), \
             unittest.mock.patch("counters.Sampler", Seen):
            with self.assertRaisesRegex(RuntimeError, "the paging counters failed"):
                counters.Counted(os.path.join(work, "paging.log"))
        self.assertTrue(made[0].done.is_set(), "stopped by the failed start")
        self.assertFalse(made[0].thread.is_alive())

    @unittest.skipUnless(sys.platform in ("darwin", "win32"), "the live counters are macOS's and Windows'")
    def test_this_system_samples_each_second(self):
        with tempfile.TemporaryDirectory() as work:
            sampler = counters.Sampler(os.path.join(work, "paging.log"), counters.reader())
            taken = until(sampler, 2)
            sampler.stop()
        self.assertGreaterEqual(len(taken), 2)
        measured = sys.platform == "darwin"
        self.assertTrue(all((si is not None) == measured and so >= 0 for _, si, so in taken), taken)



def second(at):
    return datetime.datetime(2026, 11, 20, 1, 0, at, tzinfo=datetime.timezone.utc)


class Judged(unittest.TestCase):
    def test_unmeasured_swap_in_is_judged_on_swap_out_alone(self):
        def verdict(so):
            taken = [(second(s), None, so if s == 3 else 0) for s in range(1, 8)]
            return judge.verdicts([("A", second(2), second(5))], taken, swap_in=False)[0]
        (_, quiet, note), (_, paged, _) = verdict(0), verdict(64)
        self.assertTrue(quiet)
        self.assertFalse(paged, "a swap-out voids")
        self.assertIn("si not measured", note)

    def test_unmeasured_swap_in_never_passes_where_it_is_measured(self):
        taken = [(second(s), None, 0) for s in range(1, 8)]
        with self.assertRaisesRegex(ValueError, "unmeasured"):
            judge.verdicts([("A", second(2), second(5))], taken)

    def test_windows_rising_pages_output_voids_its_case(self):
        def windows(out):
            return {"in": None, "out": out, "logged": {"page reads": 0, "pages input": 0}}

        values = iter([windows(151004)] * 3 + [windows(151068)] + [windows(151068)] * 20)
        with tempfile.TemporaryDirectory() as work, \
                unittest.mock.patch("counters.reader", lambda: lambda: next(values)), \
                unittest.mock.patch("sys.platform", "win32"):
            paging = counters.Counted(os.path.join(work, "paging.log"))
            taken = until(paging.sampler, 6)
            window = ("A", taken[0][0], taken[-2][0])
            judged = paging.judged([window])
        self.assertEqual([so for _, _, so in taken[:4]], [0, 0, 64, 0])
        self.assertEqual(judged[0][:2], ("A", False), judged)
        self.assertIn("si not measured", judged[0][2])
        self.assertFalse(paging.sampler.thread.is_alive(), "the sampler outlived its run")


if __name__ == "__main__":
    unittest.main()
