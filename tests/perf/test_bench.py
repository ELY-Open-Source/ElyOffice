#!/usr/bin/env python3
# The bench's verdicts on made-up logs and handles.
import collections
import datetime
import json
import os
import re
import sys
import tempfile
import types
import unittest
import unittest.mock

import bench
import report
import cycles
import drive
import feedback
import run

MAC = drive.Mac.system


class Log:
    """A made-up app whose log the recipe writes."""

    def __init__(self, lines):
        self.log = list(lines)

    def lines(self):
        return list(self.log)

    def wait(self, pattern, since=0, timeout=0):
        return 0


class Counting(cycles.Cycle):
    """Each close logs `after`."""

    def __init__(self, after):
        super().__init__("app", [], "ready", [r"(\d+) tabs open"])
        self.after = after

    def open(self, app, n):
        app.log.append("opened")
        return "opened"

    def close(self, app, n):
        app.log.extend(["closed", *self.after])
        return "closed"


class Verdicts(unittest.TestCase):
    def test_typing_with_a_failed_draw_fails(self):
        keys = feedback.Keys("ely-doc", [], "ready", feedback.TILE, feedback.into_document)
        typed = ["lok: edit 1 text to tile 30 ms", "lok: edit 2 text to tile 40 ms"]
        self.assertEqual(feedback.judged(keys, typed, 0, 2), ([30, 40], [], []))
        late = ["ui: the main thread was busy 150 ms (170 ms late)",
                "ui: the main thread was blocked 600 ms (ran 1 ms, waited for a CPU 2 ms)",
                "ui: the main thread was held 300 ms (ran 1 ms; this system keeps no run delay)",
                "ui: a timer came 400 ms late, the main thread ran 3 ms, waited for a CPU 390 ms: starved"]
        self.assertEqual(feedback.judged(keys, typed + late, 0, 2), ([30, 40], [150, 600, 300], [400]))
        with self.assertRaisesRegex(RuntimeError, "failed to draw"):
            feedback.judged(keys, typed + ["lok: tile paint failed: atlas full"], 0, 2)

    def test_a_cycle_needs_a_fresh_count_back_at_the_baseline(self):
        with unittest.mock.patch.object(cycles.time, "sleep"):
            cycles.one(Log(["1 tabs open"]), Counting(["1 tabs open"]), 1, (1,))
            # Only the count logged before the cycle.
            with self.assertRaisesRegex(RuntimeError, "never returned"):
                cycles.one(Log(["1 tabs open"]), Counting([]), 1, (1,))
            with self.assertRaisesRegex(RuntimeError, "never returned"):
                cycles.one(Log(["1 tabs open"]), Counting(["2 tabs open"]), 1, (1,))

    def test_a_failed_draw_voids_the_run(self):
        for line in ["lok: tile paint failed: atlas full", "lok: tile (0, 1) failed: kernel",
                     "pdf: drawing a page failed (Other)"]:
            self.assertTrue(re.search(run.PAINT_FAILED, line), line)
        self.assertFalse(re.search(run.PAINT_FAILED, "lok: first tile drawn"))

    def test_a_leak_on_the_first_close_fails(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        now = before + collections.Counter({("CHR", "/dev/null"): 1})
        with unittest.mock.patch.object(cycles, "SETTLE_S", 0.3), self.assertRaisesRegex(RuntimeError, "leaves handles"):
            cycles.handles_kept("app", before, now, 1, MAC, lambda: now)

    def test_a_handle_that_closes_within_the_wait_passes_one_that_stays_fails(self):
        # configmgr writes registrymodifications.xcu through a temp file.
        before = collections.Counter({("fd", "/p/registrymodifications.xcu"): 1})
        now = before + collections.Counter({("fd", "/p/user/h2gxTp"): 1})
        reads = iter([now, now, before])
        cycles.handles_kept("app", before, now, 264, drive.Linux.system, lambda: next(reads))
        with unittest.mock.patch.object(cycles, "SETTLE_S", 0.3), \
                self.assertRaisesRegex(RuntimeError, r"leaves handles 0.3 s on: \[\('fd', '/p/user/h2gxTp'\)\]"):
            cycles.handles_kept("app", before, now, 264, drive.Linux.system, lambda: now)

    def test_an_unnamed_socket_in_flight_is_left_to_the_trend(self):
        # A background sync's bus call, as ElyMail's on Linux.
        before = collections.Counter({("fd", "socket"): 30})
        now = before + collections.Counter({("fd", "socket"): 1})
        cycles.handles_kept("app", before, now, 123, drive.Linux.system, lambda: now)
        kept = before + collections.Counter({("fd", "/tmp/kept"): 1})
        with unittest.mock.patch.object(cycles, "SETTLE_S", 0.3), self.assertRaisesRegex(RuntimeError, "leaves handles"):
            cycles.handles_kept("app", before, kept, 1, drive.Linux.system, lambda: kept)

    def test_a_memory_climb_names_the_process_that_grew(self):
        # The engine worker grew; the app did not.
        grown = cycles.grown(10, [{10: 300, 11: 100}, {10: 300, 11: 123, 12: 5}])
        self.assertEqual(grown, {"app": (300, 300), 11: (100, 123), 12: (None, 5)})

    def test_system_resources_pass_and_their_count_must_settle(self):
        before = collections.Counter({("REG", "/tmp/a"): 1})
        font = collections.Counter({("REG", "/System/Library/Fonts/SFNSItalic.ttf"): 1})
        cycles.handles_kept("app", before, before + font, 1, MAC, lambda: before + font)
        # A driver opening its device late, then flat.
        cycles.steady("app", "handles", [37] * 2 + [43] * 148)
        with self.assertRaisesRegex(RuntimeError, "took the handles"):
            cycles.steady("app", "handles", [40 + n for n in range(40)])

    def test_unnamed_counts_may_jitter_but_not_climb(self):
        # Windows, 40 cycles after the liblangtag fix.
        measured = [1428, 1428, 1428, 1430, 1430, 1430, 1431, 1431, 1431, 1432] + [1423] * 20 + [1415, 1407, 1407] + [1399] * 7
        cycles.handles_steady("app", 1423, measured)
        with self.assertRaisesRegex(RuntimeError, "took the handles"):
            cycles.handles_steady("app", 1365, [1365, 1371, 1377, 1383, 1389])
        # One handle every second cycle, 19 in all.
        with self.assertRaisesRegex(RuntimeError, "took the handles"):
            cycles.handles_steady("app", 1400, [1400 + n // 2 for n in range(40)])
        # The VM's pool: up 11, back down, settling 9 high.
        swings = [1074] * 30 + [1066] * 30 + [1083] * 30 + [1076] * 30 + [1081] * 30
        cycles.handles_steady("app", 1072, swings)
        with self.assertRaisesRegex(RuntimeError, "keep 13 handles"):
            cycles.handles_steady("app", 1072, [1085] * 40)
        # The first cycle keeps 50 and never gives them back.
        with self.assertRaisesRegex(RuntimeError, "keep 50 handles"):
            cycles.handles_steady("app", 1400, [1450] * 40)

    def test_memory_may_wobble_but_not_climb(self):
        cycles.steady("app", "MB", [261, 261, 254, 255, 254, 261, 260, 261, 255, 256] * 2, cycles.PAGE_MB)
        # ElyPDF on Linux: thread arenas keep 55 MB, then flat.
        arenas = [
            234.6, 236.7, 240.6, 244.0, 245.4, 246.8, 248.7, 248.7, 250.6, 252.0,
            253.9, 255.9, 255.9, 257.9, 261.8, 262.5, 262.6, 266.5, 266.6, 268.6,
            268.6, 270.5, 270.5, 271.2, 271.9, 271.9, 272.6, 274.6, 274.6, 274.6,
            275.6, 275.6, 276.3,
        ] + [276.3] * 66 + [277.0]
        cycles.steady("app", "MB", arenas, cycles.PAGE_MB)
        # ElyDoc on Linux, 100 cycles: warm-up, then flat.
        settling = [
            387.0, 387.6, 388.0, 388.9, 389.1, 389.3, 389.4, 389.5, 390.0, 390.2,
            390.3, 390.4, 390.6, 390.2, 390.4, 390.6, 391.3, 391.3, 391.5, 391.5,
            391.5, 391.7, 391.7, 391.8, 391.9, 392.0, 392.0, 392.0, 392.7, 392.8,
            392.8, 392.3, 392.5, 392.5, 392.7, 392.7, 392.8, 392.8, 392.8, 392.8,
            392.8, 392.9, 392.9, 394.3, 394.6, 394.6, 395.1, 395.1, 394.9, 394.9,
            395.0, 394.7, 395.1, 394.6, 394.7, 394.7, 394.3, 394.3, 394.3, 394.3,
            394.4, 393.9, 394.1, 394.1, 394.2, 394.8, 395.0, 395.0, 395.0, 395.5,
            395.6, 395.9, 395.9, 396.1, 396.2, 396.1, 396.1, 396.1, 396.2, 396.3,
            396.3, 396.3, 396.3, 396.3, 396.3, 396.4, 396.4, 396.4, 396.3, 396.3,
            396.1, 396.1, 396.1, 395.9, 395.6, 395.6, 396.1, 395.6, 395.6, 395.1,
        ]
        cycles.steady("app", "MB", settling, cycles.PAGE_MB)
        # ElyMark on Linux, 150 cycles: under a page a cycle.
        runs = [(120, 7), (125, 4), (127, 3), (128, 3), (129, 3), (130, 1), (131, 2), (132, 2), (134, 1), (135, 7), (136, 1),
                (139, 3), (140, 3), (141, 2), (142, 2), (143, 6), (144, 4), (145, 3), (147, 2), (149, 6), (150, 3), (151, 6)]
        tail = [173 + k / 256 for k, n in runs for _ in range(n)]
        cycles.steady("app", "MB", [173.4] * 76 + tail, cycles.PAGE_MB)
        # ElyTable on Linux: the window's heap steps, then flat.
        stepped = [435.2] * 76 + [436.8] * 5 + [448.8] * 4 + [456.1] * 11 + [457.96] * 17 + [458.0 + n / 768 for n in range(37)]
        cycles.steady("app", "MB", stepped, cycles.PAGE_MB)
        with self.assertRaisesRegex(RuntimeError, "took the MB"):
            cycles.steady("app", "MB", [373 + n for n in range(20)], cycles.PAGE_MB)
        # A tenth of a MB every cycle.
        with self.assertRaisesRegex(RuntimeError, "took the MB"):
            cycles.steady("app", "MB", [387 + n / 10 for n in range(100)], cycles.PAGE_MB)
        # 8 KiB a cycle; the app frees 264 KiB last.
        leak = [300 + n * 8 / 1024 for n in range(150)]
        leak[-1] -= 264 / 1024
        with self.assertRaisesRegex(RuntimeError, "took the MB"):
            cycles.steady("app", "MB", leak, cycles.PAGE_MB)

    def test_sockets_kept_every_other_cycle_fail(self):
        before = collections.Counter({("fd", "socket"): 30, ("fd", "/tmp/a"): 1})
        self.assertEqual(cycles.unnamed(before), 30)
        with self.assertRaisesRegex(RuntimeError, "keep 57 handles"):
            cycles.handles_steady("app", 30, [30 + (n + 1) // 2 for n in range(1, 151)])

    def test_counts_come_from_the_log(self):
        self.assertEqual(cycles.counted(["x", "lok-worker: 2 documents open", "lok-worker: 1 documents open"],
                                        r"lok-worker: (\d+) documents open"), 1)
        with self.assertRaisesRegex(RuntimeError, "no count"):
            cycles.counted(["office: closed Untitled 2, 1 tabs open"], r"lok-worker: (\d+) documents open")

    def test_an_answer_is_timed_by_its_stamp_not_its_reading(self):
        log = ["[2026-10-29T01:11:42.100Z INFO  ely_tree] tree: opened /r",
               "[2026-10-29T01:11:42.730Z DEBUG ely_tree] history shows 50 commits, searched false"]
        sent = run.logged_at(log[0])
        # However late the driver returns, the stamps decide.
        self.assertEqual(run.answered(log, "history shows", sent), 630)
        self.assertIsNone(run.answered(log, "searched true", sent))
        with self.assertRaisesRegex(RuntimeError, "before its input"):
            run.answered(log, "history shows", sent + 1000)
        with self.assertRaisesRegex(RuntimeError, "millisecond stamp"):
            run.logged_at("[2026-10-29T01:11:42Z INFO  x] no milliseconds")

    def test_a_helper_must_say_when_it_sent(self):
        self.assertEqual(drive.sent("drive: took the front back\nsent 1761700302730\n"), 1761700302730)
        with self.assertRaisesRegex(RuntimeError, "did not say"):
            drive.sent("pressed\n")

    def test_ElyMark_memory_is_held_to_its_budget(self):
        env = {"date": "2000-01-01", "machine": "Test"}
        rows = [("PERF-07", "ElyMark at the budget", [100], 400.0),
                ("PERF-07", "ElyMark past it", [100], 401.0)]
        held = [("ely-mark", (120.0, 50, (1,)), [(121.0, 50, (1,))])]
        with tempfile.TemporaryDirectory() as out:
            report.write(out, env, rows, held, 1)
            with open(os.path.join(out, "2000-01-01-Test.md"), encoding="utf-8") as record:
                text = record.read()
        self.assertIn("| ElyMark at the budget | 1 | 100 | 100 ✓ | 2000 | 400 ✓ | 400 |", text)
        self.assertIn("| ElyMark past it | 1 | 100 | 100 ✓ | 2000 | 401 ✗ | 400 |", text)
        self.assertIn("before: 120/50/1", text)

    def test_starved_timers_are_listed_not_judged(self):
        env = {"date": "2000-01-01", "machine": "Test"}
        rows = [("PERF-02", "ElyMail timers late >100 ms while starved: 1 times", [601], None),
                ("PERF-02", "ElyMail key to frame", [601], None)]
        with tempfile.TemporaryDirectory() as out:
            report.write(out, env, rows, [], 1)
            with open(os.path.join(out, "2000-01-01-Test.md"), encoding="utf-8") as record:
                text = record.read()
        self.assertIn("| ElyMail timers late >100 ms while starved: 1 times | 1 | 601 | 601 | 100 |", text)
        self.assertIn("| ElyMail key to frame | 1 | 601 | 601 ✗ | 100 |", text)


class Children(drive.Host):
    """App 1 with child 2; `alive` says if 2 runs."""

    def __init__(self, alive):
        self.alive = alive

    def pids(self, root):
        return {1, 2} if self.alive else {1}

    def held(self, pid):
        if pid == 1:
            return collections.Counter({("fd", "x"): 3})
        raise RuntimeError("process 2: no such process")


class Ended(unittest.TestCase):
    def test_a_child_that_exited_is_left_out(self):
        self.assertEqual(Children(False).handles(1, [1, 2]), collections.Counter({("fd", "x"): 3}))

    def test_a_live_child_that_cannot_be_read_voids_the_run(self):
        with self.assertRaises(RuntimeError):
            Children(True).handles(1, [1, 2])

    def test_the_app_itself_is_never_left_out(self):
        host = Children(False)
        host.held = lambda pid: (_ for _ in ()).throw(RuntimeError("gone"))
        with self.assertRaises(RuntimeError):
            host.handles(1, [1])



class Exiting(drive.Host):
    """Child 2 is listed, then exits before its read."""

    listed = True

    def pids(self, root):
        listed, self.listed = self.listed, False
        return {1, 2} if listed else {1}

    def private_kb(self, pid):
        if pid == 2:
            raise OSError("process 2: no such process")
        return 100


class Private(unittest.TestCase):
    def test_macos_reads_the_footprint_ledger_not_resident(self):
        def rusage(pid, flavor, info):
            # rusage_info_v2 in uint64s: uuid 0-1, ..., resident 8, footprint 9.
            for at in range(len(info._obj)):
                info._obj[at] = (at + 1) << 20
            return 0

        host = drive.Mac.__new__(drive.Mac)
        host.libsystem = types.SimpleNamespace(proc_pid_rusage=rusage)
        self.assertEqual(host.private_kb(1), 10 << 10, "ri_phys_footprint, in KiB")

    def test_a_child_gone_before_its_read_is_left_out(self):
        self.assertEqual(Exiting().private(1), {1: 100})

    def test_a_live_child_that_cannot_be_read_voids_the_run(self):
        host = Exiting()
        host.pids = lambda root: {1, 2}
        with self.assertRaises(OSError):
            host.private(1)

    def test_this_platform_counts_memory_the_process_holds(self):
        cls = {"linux": drive.Linux, "darwin": drive.Mac, "win32": drive.Windows}[sys.platform]
        host = cls.__new__(cls)
        before = host.private_kb(os.getpid())
        held = bytearray(64 * 2**20)
        # Written, so every page is the process's own.
        held[::4096] = b"x" * len(held[::4096])
        grown = (host.private_kb(os.getpid()) - before) / 1024
        self.assertGreater(grown, 60, "64 MB held, written")
        del held


class Kernel:
    """Toolhelp that lists `count` processes, then stops with `error`."""

    def __init__(self, count, error):
        self.count, self.error, self.given = count, error, 0
        self.CreateToolhelp32Snapshot = lambda flags, pid: 7
        self.Process32NextW = self.Process32FirstW
        self.CloseHandle = lambda snap: 1

    def Process32FirstW(self, snap, entry):
        if self.given == self.count:
            drive.ctypes.set_last_error(self.error)
            return 0
        self.given += 1
        entry._obj.pid, entry._obj.parent = self.given, 0
        return 1


@unittest.skipUnless(os.name == "nt", "Toolhelp is Windows only")
class Listing(unittest.TestCase):
    def listed(self, count, error):
        host = object.__new__(drive.Windows)
        host.k32 = Kernel(count, error)
        return host.processes()

    def test_a_whole_list_ends_with_no_more_files(self):
        self.assertEqual(self.listed(2, 18), [(1, 0), (2, 0)])

    def test_a_failed_first_entry_voids_the_run(self):
        with self.assertRaises(RuntimeError):
            self.listed(0, 5)

    def test_a_list_cut_short_voids_the_run(self):
        with self.assertRaises(RuntimeError):
            self.listed(1, 5)


class Windows(unittest.TestCase):
    def test_a_failed_redo_keeps_what_it_finished_and_leaves_the_rest_void(self):
        rows = lambda n: {"rows": [["PERF-01", f"case {n}", [n], None]]}
        for a_counts, void in [(True, ["B", "C"]), (False, ["A", "B", "C"])]:
            kept = {"env": {"date": "2000-01-01", "machine": "Test", "void": ["A", "B", "C"]},
                    "cases": {"A": rows(1), "B": rows(2), "C": rows(3)}}
            cases = bench.Cases(json.loads(json.dumps(kept["cases"])), ["A", "B"])
            with unittest.mock.patch("builtins.print"):
                cases.run("A", lambda _: rows(10)["rows"])
                with self.assertRaises(RuntimeError):
                    cases.run("B", lambda _: (_ for _ in ()).throw(RuntimeError("cycle 264")))
            with tempfile.TemporaryDirectory() as out, tempfile.TemporaryDirectory() as work:
                trace = os.path.join(work, "vmstat.log")
                with open(trace, "w", encoding="utf-8") as samples:
                    samples.write("the window's samples\n")
                paging = types.SimpleNamespace(judged=lambda windows: [(key, a_counts, "") for key, _, _ in windows],
                                               path=trace, suffix="vmstat")
                args = types.SimpleNamespace(out=out, keep=os.path.join(out, "raw.json"), runs=1)
                held = {"judged": None, "keep": None, "traced": False, "raw": False, "stranded": False, "trace": (trace, "vmstat")}
                # The failure's log copy lands in this test's folder.
                with unittest.mock.patch("builtins.print"), unittest.mock.patch.object(tempfile, "tempdir", work):
                    bench.failed(args, kept, cases, paging, held, lambda: {"date": "2000-01-02", "machine": "Test"},
                                 RuntimeError("cycle 264"), lambda: None, work)
                with open(args.keep, encoding="utf-8") as raw:
                    saved = json.load(raw)
                with open(os.path.join(out, "2000-01-01-Test.md"), encoding="utf-8") as record:
                    self.assertIn("| PERF-01 | case 10 |", record.read())
                # A kept case's verdict has its samples beside --keep.
                traces = [name for name in os.listdir(out) if re.fullmatch(r"raw\.json\.\d{8}T\d{6}\.vmstat", name)]
                self.assertEqual(len(traces), 1, os.listdir(out))
                with open(os.path.join(out, traces[0]), encoding="utf-8") as copied:
                    self.assertEqual(copied.read(), "the window's samples\n")
            self.assertEqual(saved["cases"]["A"], rows(10))
            self.assertEqual(saved["cases"]["B"], rows(2))
            self.assertEqual(saved["env"]["void"], void)
            self.assertEqual(list(saved["env"]["rerun"]), ["A"])

    def test_a_failed_redo_on_another_machine_keeps_nothing(self):
        kept = {"env": {"date": "2000-01-01", "machine": "Test", "void": ["A"]}, "cases": {"A": {"rows": []}}}
        cases = bench.Cases(kept["cases"], ["A"])
        with unittest.mock.patch("builtins.print"):
            cases.run("A", lambda _: [["PERF-01", "case", [1], None]])
        with tempfile.TemporaryDirectory() as out:
            args = types.SimpleNamespace(out=out, keep=os.path.join(out, "raw.json"), runs=1)
            with self.assertRaisesRegex(RuntimeError, "same builds, samples and machine"):
                bench.failed_redo(args, kept, cases, {"judged": [("A", True, "")]}, lambda: {"date": "2000-01-02", "machine": "Other"})
            self.assertEqual(os.listdir(out), [])

    def test_a_redo_on_other_builds_or_samples_is_refused_before_it_runs(self):
        env = {"builds": {"ely-doc": "x"}, "samples": {"doc.docx": "s"}}
        bench.matched(env, {"ely-doc": "x"}, {"doc.docx": ("/p", "s")})
        for builds, found in [({"ely-doc": "y"}, {"doc.docx": ("/p", "s")}), ({"ely-doc": "x"}, {"doc.docx": ("/p", "t")})]:
            with self.assertRaisesRegex(RuntimeError, "same builds and samples"):
                bench.matched(env, builds, found)

    def test_a_redo_naming_an_unknown_case_measures_nothing(self):
        kept = {"env": {"date": "2000-01-01", "machine": "Test"}, "cases": {"PERF-01 ely-doc": {"rows": []}}}
        with tempfile.TemporaryDirectory() as out:
            keep = os.path.join(out, "raw.json")
            with open(keep, "w", encoding="utf-8") as raw:
                json.dump(kept, raw)
            argv = ["bench.py", "--keep", keep, "--out", out, "--redo", "PERF-01 ely-doc", "PERF-01 ely-nope"]
            touched = []
            with unittest.mock.patch.object(sys, "argv", argv), \
                 unittest.mock.patch.object(bench.drive, "sh", lambda *a, **k: touched.append(("build", a))), \
                 unittest.mock.patch.object(bench.drive, "host", lambda *a: touched.append(("host", a))), \
                 unittest.mock.patch.object(bench.drive, "frames_apart", lambda *a: touched.append(("frames", a))), \
                 unittest.mock.patch.object(bench.Cases, "run", lambda *a, **k: touched.append(("case", a[1]))):
                with self.assertRaisesRegex(RuntimeError, r"no such case to redo: \['PERF-01 ely-nope'\]"):
                    bench.main()
            self.assertEqual(touched, [], "nothing built or measured")
            with open(keep, encoding="utf-8") as raw:
                self.assertEqual(json.load(raw), kept, "--keep is untouched")
            self.assertEqual(os.listdir(out), ["raw.json"])

    def test_a_redo_runs_only_its_case_after_its_setup_and_prints_its_window(self):
        done = {"A": {"rows": [["P", "a", [1], None]]}, "B": {"rows": [["P", "b", [2], None]]}}
        cases, order = bench.Cases(done, ["B"]), []
        clock = lambda: order.append("clock") or datetime.datetime(2026, 11, 19, tzinfo=datetime.timezone.utc)
        with unittest.mock.patch("builtins.print") as said, unittest.mock.patch.object(bench, "utc", clock):
            cases.run("A", lambda _: order.append("A produced"))
            cases.run("B", lambda home: order.append(f"B produced in {home}") or [["P", "b", [3], None]],
                      setup=lambda: order.append("B set up") or "seed")
        self.assertEqual(order, ["B set up", "clock", "B produced in seed", "clock"])
        self.assertEqual(list(done), ["A", "B"])
        self.assertEqual(done["A"]["rows"][0][2], [1])
        self.assertEqual(done["B"]["rows"][0][2], [3])
        self.assertEqual(cases.ran, {"B"})
        line = said.call_args.args[0]
        self.assertRegex(line, r"^bench: case B from \S+\+00:00 to \S+\+00:00$")

    def test_a_seeded_home_is_made_once_before_the_window(self):
        made = []
        recipe = types.SimpleNamespace(home=lambda: made.append(1) or f"home{len(made)}")
        bench.pinned(recipe)
        self.assertEqual((recipe.home(), recipe.home(), len(made)), ("home1", "home1", 1))
        plain = types.SimpleNamespace(home=None)
        bench.pinned(plain)
        self.assertIsNone(plain.home)

    def test_a_redo_keeps_the_first_record_and_names_what_it_redid(self):
        before = {"date": "2026-11-19", "builds": {"ely-doc": "x"}, "rerun": {"A": "t"}}
        env = bench.redone(before, {"date": "2026-11-20", "builds": {"ely-doc": "x"}}, ["B"])
        self.assertEqual(env["date"], "2026-11-19")
        self.assertEqual(set(env["rerun"]), {"A", "B"})
        with self.assertRaises(RuntimeError):
            bench.redone(before, {"date": "2026-11-20", "builds": {"ely-doc": "y"}}, ["B"])


if __name__ == "__main__":
    unittest.main()
