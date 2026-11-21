"""A void or failed run keeps its samples without --keep."""

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


class Kept(unittest.TestCase):
    def ran(self, verdict, stopped=None, machine=lambda found, partial: {"date": "2000-01-01", "machine": "Test"}, more=lambda scratch: (), left=()):
        """bench's main, every build and case stubbed; its output."""
        Trace.verdict, Trace.stopped = verdict, stopped
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        parent = os.path.join(scratch.name, "docs")
        os.makedirs(parent)
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
            try:
                bench.main()
            except (SystemExit, RuntimeError) as error:
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

    def test_a_clean_run_without_keep_leaves_nothing_extra(self):
        parent, said, ended = self.ran([(KEY, True, "")])
        self.assertIsNone(ended)
        self.assertEqual(os.listdir(parent), [])
        self.assertFalse([line for line in said if "raw results" in line], said)


if __name__ == "__main__":
    unittest.main()
