"""A void or failed run keeps its samples without --keep."""

import contextlib
import datetime
import itertools
import json
import os
import re
import sys
import tempfile
import types
import unittest
import unittest.mock

import bench

KEY = "PERF-01 ely-doc"


class Trace:
    """bench's Paging, its samples written and its verdict given."""

    suffix = "vmstat"
    verdict = stopped = None

    def __init__(self, path):
        self.path = path
        with open(path, "w", encoding="utf-8") as trace:
            trace.write("the run's samples\n")

    def judged(self, windows):
        if isinstance(self.verdict, Exception):
            raise self.verdict
        return self.verdict

    def stop(self):
        if self.stopped:
            raise self.stopped


class Silent(Trace):
    """A sampler that starts, writes a little, and never samples."""

    def __init__(self, path):
        with open(path, "w", encoding="utf-8") as trace:
            trace.write("the run's samples\n")
        raise RuntimeError("vmstat gave no samples in 10 s (exit None)")


class Kept(unittest.TestCase):
    def ran(self, verdict, stopped=None, machine=lambda found, partial: {"date": "2000-01-01", "machine": "Test"}, more=lambda scratch: (), left=(), patches=(), prepare=lambda parent: None):
        """bench's main, every build and case stubbed; its output."""
        Trace.verdict, Trace.stopped = verdict, stopped
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        parent = os.path.join(scratch.name, "docs")
        os.makedirs(parent)
        prepare(parent)
        host = types.SimpleNamespace(name="Linux", git=lambda: "git")
        said, ended = [], None
        self.scratch = scratch.name
        argv = ["bench.py", "--out", os.path.join(parent, "perf"), "--only", "none", *more(scratch.name)]
        with unittest.mock.patch.object(sys, "argv", argv), \
             unittest.mock.patch.object(tempfile, "tempdir", scratch.name), \
             unittest.mock.patch.object(bench.run, "HOST", None), \
             unittest.mock.patch.object(bench.drive, "host", lambda work: host), \
             unittest.mock.patch.object(bench.drive, "sh", lambda *a, **k: None), \
             unittest.mock.patch.object(bench.drive, "frames_apart", lambda *a: None), \
             unittest.mock.patch.object(bench.samples, "made", lambda *a: {}), \
             unittest.mock.patch.object(bench.run, "stop_engines", lambda work: list(left)), \
             unittest.mock.patch.object(bench, "environment", machine), \
             unittest.mock.patch.object(bench, "write", lambda *a: None), \
             unittest.mock.patch.object(bench, "built", lambda: {}), \
             unittest.mock.patch.object(bench, "matched", lambda *a: None), \
             unittest.mock.patch.object(bench, "Paging", Trace), \
             unittest.mock.patch("builtins.print", lambda *a, **k: said.append(" ".join(map(str, a)))):
            with contextlib.ExitStack() as stack:
                for where, name, value in patches:
                    stack.enter_context(unittest.mock.patch.object(where, name, value))
                try:
                    bench.main()
                except (SystemExit, Exception) as error:
                    ended = error
        return parent, said, ended

    def assert_kept(self, parent, path, void):
        self.assertEqual(os.path.dirname(path), os.path.join(parent, "bench-kept"), "beside --out, not in it")
        with open(path, encoding="utf-8") as raw:
            env = json.load(raw)["env"]
        self.assertEqual(env["void"], void)
        traces = [name for name in os.listdir(os.path.dirname(path))
                  if re.fullmatch(re.escape(os.path.basename(path)) + r"\.\d{8}T\d{6}\.vmstat", name)]
        self.assertEqual(len(traces), 1, os.listdir(os.path.dirname(path)))
        self.assertEqual(len(os.listdir(os.path.dirname(path))), 2, "no partial file left")
        with open(os.path.join(os.path.dirname(path), traces[0]), encoding="utf-8") as trace:
            self.assertEqual(trace.read(), "the run's samples\n")
        self.assertEqual(os.listdir(parent), ["bench-kept"], "nothing under --out")

    def test_a_void_run_without_keep_leaves_its_samples_at_the_printed_path(self):
        parent, _, ended = self.ran([(KEY, False, "paged out")])
        self.assertIsInstance(ended, SystemExit)
        path = re.search(r"samples and raw results: (\S+)$", str(ended.code)).group(1)
        self.assert_kept(parent, path, [KEY])

    def test_a_failed_run_without_keep_leaves_its_samples_at_the_printed_path(self):
        parent, said, ended = self.ran(RuntimeError("vmstat stopped"))
        self.assertIsInstance(ended, RuntimeError)
        printed = [line for line in said if line.startswith("bench: the failed run's samples and raw results: ")]
        self.assertEqual(len(printed), 1, said)
        self.assert_kept(parent, printed[0].rsplit(" ", 1)[1], [])

    def failed_path(self, said):
        printed = [line for line in said if line.startswith("bench: the failed run's samples and raw results: ")]
        self.assertEqual(len(printed), 1, said)
        return printed[0].rsplit(" ", 1)[1]

    def test_an_early_failure_before_the_gpu_is_known_keeps_valid_results_and_its_error(self):
        def unread(found, partial):
            raise SystemExit("no app logged the GPU it chose")
        parent, said, ended = self.ran(RuntimeError("the first start failed"), machine=unread)
        self.assertIsInstance(ended, RuntimeError)
        self.assertEqual(str(ended), "the first start failed")
        path = self.failed_path(said)
        self.assert_kept(parent, path, [])
        with open(path, encoding="utf-8") as raw:
            self.assertIn("no app logged the GPU it chose", json.load(raw)["env"]["unread"])

    def test_a_sampler_that_will_not_stop_never_hides_the_failure(self):
        parent, said, ended = self.ran(RuntimeError("original judgment failed"), stopped=OSError("sampler stop failed"))
        self.assertIsInstance(ended, RuntimeError)
        self.assertEqual(str(ended), "original judgment failed")
        shown = "bench: the sampler did not stop: OSError: sampler stop failed"
        self.assertIn(shown, said)
        self.assertIn(shown, ended.__notes__)
        self.assert_kept(parent, self.failed_path(said), [])

    def test_a_redo_failing_before_any_case_leaves_keep_and_its_void(self):
        kept = {"env": {"date": "2000-01-01", "machine": "Test", "void": [KEY]}, "cases": {KEY: {"rows": []}}}
        def redo(scratch):
            keep = os.path.join(scratch, "raw.json")
            with open(keep, "w", encoding="utf-8") as raw:
                json.dump(kept, raw)
            return ["--keep", keep, "--redo", KEY]
        # --only runs nothing: the redo fails with no case done.
        _, said, ended = self.ran(RuntimeError("the sampler gave no verdict"), more=redo)
        self.assertIsInstance(ended, RuntimeError)
        self.assertTrue(str(ended).startswith("no such case to redo here"), ended)
        keep = os.path.join(self.scratch, "raw.json")
        self.assertIn(f"bench: the failed run's samples and raw results: {keep}", said)
        with open(keep, encoding="utf-8") as raw:
            self.assertEqual(json.load(raw), kept, "the unmeasured case stays void")
        traces = [name for name in os.listdir(self.scratch) if re.fullmatch(r"raw\.json\.\d{8}T\d{6}\.vmstat", name)]
        self.assertEqual(len(traces), 1, os.listdir(self.scratch))

    def test_a_failure_after_judging_keeps_samples_and_results(self):
        def late(found, partial):
            raise RuntimeError("final metadata read failed")
        parent, said, ended = self.ran([(KEY, True, "")], machine=late)
        self.assertIsInstance(ended, RuntimeError)
        self.assertEqual(str(ended), "final metadata read failed")
        path = self.failed_path(said)
        self.assert_kept(parent, path, [])
        with open(path, encoding="utf-8") as raw:
            self.assertIn("final metadata read failed", json.load(raw)["env"]["unread"])

    def test_engines_that_outlive_the_run_keep_samples_and_results(self):
        parent, said, ended = self.ran([(KEY, True, "")], left=["ely-doc engine stop: exit 1"])
        self.assertIsInstance(ended, RuntimeError)
        self.assertIn("engines outlived the run", str(ended))
        self.assert_kept(parent, self.failed_path(said), [])

    def test_a_redo_on_another_machine_leaves_keep_as_it_was(self):
        tree = "PERF-06 ely-tree"
        kept = {"env": {"date": "2000-01-01", "machine": "Test", "void": [tree]}, "cases": {tree: {"rows": []}}}
        def redo(scratch):
            self.keep = os.path.join(scratch, "raw.json")
            with open(self.keep, "w", encoding="utf-8") as raw:
                json.dump(kept, raw)
            with open(self.keep, "rb") as raw:
                self.before = raw.read()
            return ["--only", "PERF-06", "--keep", self.keep, "--redo", tree]
        other = lambda found, partial: {"date": "2000-01-02", "machine": "Other"}
        patches = [(bench.samples, "made", lambda *a: {"repo": ("/repo", "s")}),
                   (bench, "searched_tree", lambda work, repo, runs: [("PERF-06", "tree", [1], None)])]
        _, said, ended = self.ran([(tree, True, "")], machine=other, more=redo, patches=patches)
        self.assertIsInstance(ended, RuntimeError)
        self.assertIn("same builds, samples and machine", str(ended))
        # The redo's own results fit nowhere: said, folder kept.
        stays = f"bench: raw results not kept at {self.keep}; the work folder stays: "
        stayed = [line for line in said if line.startswith(stays)]
        self.assertEqual(len(stayed), 1, said)
        self.assertEqual(self.works(), [os.path.basename(stayed[0].rsplit(" ", 1)[1])])
        with open(self.keep, "rb") as raw:
            self.assertEqual(raw.read(), self.before, "--keep byte for byte")
        traces = [name for name in os.listdir(self.scratch) if re.fullmatch(r"raw\.json\.\d{8}T\d{6}\.vmstat", name)]
        self.assertEqual(len(traces), 1, os.listdir(self.scratch))

    def test_a_record_that_cannot_be_written_keeps_samples_and_results(self):
        def refused(*a):
            raise OSError("the record could not be written")
        parent, said, ended = self.ran([(KEY, True, "")], patches=[(bench, "write", refused)])
        self.assertIsInstance(ended, OSError)
        self.assertEqual(str(ended), "the record could not be written")
        self.assert_kept(parent, self.failed_path(said), [])

    def test_a_trace_copy_or_raw_write_that_fails_once_is_kept_by_the_failure(self):
        for name in ("kept_trace", "written"):
            with self.subTest(name):
                real, calls = getattr(bench, name), []
                def flaky(*a, real=real, calls=calls):
                    calls.append(a)
                    if len(calls) == 1:
                        raise OSError("a transient write failure")
                    return real(*a)
                # A second a call: a second keep path would show.
                ticks = itertools.count()
                clock = lambda: datetime.datetime(2026, 11, 21, tzinfo=datetime.timezone.utc) + datetime.timedelta(seconds=next(ticks))
                parent, said, ended = self.ran([(KEY, False, "paged")], patches=[(bench, name, flaky), (bench, "utc", clock)])
                self.assertIsInstance(ended, OSError)
                self.assertEqual(str(ended), "a transient write failure")
                self.assert_kept(parent, self.failed_path(said), [])

    def works(self):
        return [name for name in os.listdir(self.scratch) if name.startswith("tmp")]

    def test_a_sampler_that_never_starts_keeps_its_trace_and_cleans_up(self):
        parent, said, ended = self.ran(None, patches=[(bench, "Paging", Silent)])
        self.assertIsInstance(ended, RuntimeError)
        self.assertTrue(str(ended).startswith("vmstat gave no samples"), ended)
        self.assert_kept(parent, self.failed_path(said), [])
        self.assertEqual(self.works(), [], "the work folder is gone")

    def test_a_log_copy_that_fails_never_hides_the_failure(self):
        real, refusals = bench.shutil.copy, []
        def refused(log, to):
            if "ely-bench-failed-" in to:
                refusals.append(log)
                raise OSError("log copy refused")
            return real(log, to)
        parent, said, ended = self.ran(RuntimeError("judgment failed"), patches=[(bench.shutil, "copy", refused)])
        self.assertEqual([os.path.basename(log) for log in refusals], ["vmstat.log"], "the copy itself refused")
        self.assertIsInstance(ended, RuntimeError)
        self.assertEqual(str(ended), "judgment failed")
        self.assertIn("bench: the logs were not copied: OSError: log copy refused", ended.__notes__)
        self.assert_kept(parent, self.failed_path(said), [])

    @unittest.skipIf(not hasattr(os, "geteuid") or os.geteuid() == 0, "needs a folder the user cannot write")
    def test_no_folder_for_the_evidence_keeps_the_work_folder_and_the_failure(self):
        def locked(parent):
            os.chmod(parent, 0o500)
            self.addCleanup(os.chmod, parent, 0o700)
        parent, said, ended = self.ran(RuntimeError("judgment failed"), prepare=locked)
        self.assertIsInstance(ended, RuntimeError)
        self.assertEqual(str(ended), "judgment failed")
        self.assertTrue(any(note.startswith("bench: no folder could hold the evidence: PermissionError") for note in ended.__notes__),
                        ended.__notes__)
        stays = [line for line in said if line.startswith("bench: nothing could be kept; the work folder stays: ")]
        self.assertEqual(len(stays), 1, said)
        work = stays[0].rsplit(" ", 1)[1]
        self.assertIn(stays[0], ended.__notes__)
        with open(os.path.join(work, "vmstat.log"), encoding="utf-8") as trace:
            self.assertEqual(trace.read(), "the run's samples\n")
        self.assertEqual(self.works(), [os.path.basename(work)])
        self.assertEqual(os.listdir(parent), [], "no bench-kept")

    def vmstat_exits(self, refusal=None):
        """Paging over a vmstat that exits at once; its error."""
        class Exited:
            def __init__(self, *a, **k):
                self.stopped = []
            def poll(self):
                return 1
            def terminate(self):
                self.stopped.append("terminate")
                if refusal:
                    raise refusal
            def wait(self):
                self.stopped.append("wait")
        made = []
        with tempfile.TemporaryDirectory() as work, \
             unittest.mock.patch("builtins.print"), \
             unittest.mock.patch.object(bench.subprocess, "Popen", lambda *a, **k: made.append(Exited()) or made[-1]):
            with self.assertRaisesRegex(RuntimeError, "vmstat gave no samples") as raised:
                bench.Paging(os.path.join(work, "vmstat.log"))
        return made[0].stopped, raised.exception

    def test_vmstat_that_starts_but_never_samples_is_stopped(self):
        self.assertEqual(self.vmstat_exits()[0], ["terminate", "wait"])

    def test_a_vmstat_that_will_not_stop_never_hides_why_it_started_badly(self):
        stopped, error = self.vmstat_exits(OSError("terminate refused"))
        self.assertEqual(stopped, ["terminate"])
        self.assertEqual(error.__notes__, ["bench: the sampler did not stop: OSError: terminate refused"])

    def test_an_unusable_keep_keeps_the_work_folder_and_says_so(self):
        keep = lambda scratch: os.path.join(scratch, "missing", "raw.json")
        parent, said, ended = self.ran(RuntimeError("judgment failed"), more=lambda scratch: ["--keep", keep(scratch)])
        self.assertIsInstance(ended, RuntimeError)
        self.assertEqual(str(ended), "judgment failed")
        for what in ("samples", "raw results"):
            self.assertTrue(any(note.startswith(f"bench: the {what} were not kept: FileNotFoundError") for note in ended.__notes__),
                            ended.__notes__)
        self.assertFalse([line for line in said if line.startswith("bench: the failed run's samples")], said)
        stays = f"bench: samples and raw results not kept at {keep(self.scratch)}; the work folder stays: "
        stayed = [line for line in said if line.startswith(stays)]
        self.assertEqual(len(stayed), 1, said)
        self.assertIn(stayed[0], ended.__notes__)
        work = stayed[0].rsplit(" ", 1)[1]
        with open(os.path.join(work, "vmstat.log"), encoding="utf-8") as trace:
            self.assertEqual(trace.read(), "the run's samples\n")
        self.assertEqual(self.works(), [os.path.basename(work)])

    def test_a_clean_run_without_keep_leaves_nothing_extra(self):
        parent, said, ended = self.ran([(KEY, True, "")])
        self.assertIsNone(ended)
        self.assertEqual(os.listdir(parent), [])
        self.assertFalse([line for line in said if "raw results" in line], said)


if __name__ == "__main__":
    unittest.main()
