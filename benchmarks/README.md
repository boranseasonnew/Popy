# Repeatable benchmark suites

A suite runs a reviewed list of cases sequentially. Each case can repeat 1–3 times; every repeat gets its own ordinary Popy benchmark report. A suite contains at most 12 cases and 24 jobs. Model and color settings are explicit.

```powershell
.\scripts\dev.ps1 run -- suite --manifest benchmarks/technical.json --dry-run
.\scripts\dev.ps1 run -- fixtures --out fixtures/generated/suite-demo
.\scripts\dev.ps1 run -- suite --manifest benchmarks/technical.json --out runs/suite-01
```

The dry run needs no installed tools or models. It validates the manifest and shows whether input files exist. It does not create an output directory. Actual execution requires the local tools, all input files and a new output directory.

The example processes the first **one second** of the generated 10-second clips. It is a short integration/timing check, not a 10-second benchmark or a restoration-quality score. Change `seconds` explicitly for a longer comparison; the per-job limits still apply.

## Manifest format

```json
{
  "schema_version": 1,
  "cases": [{
    "id": "example-144p",
    "input": "../fixtures/generated/suite-demo/input-144p.mp4",
    "model": "anime",
    "seconds": 1,
    "rights": "Documented source usage rights",
    "repeats": 3,
    "gpu": 0,
    "tile": 64,
    "auto_color": false
  }]
}
```

Paths resolve relative to the manifest file. Case IDs must be unique and contain only ASCII letters, digits, hyphens or underscores. Unknown fields are rejected so an option typo does not silently change the test. Defaults: one repeat, GPU 0, tile 64, color off. Models: `anime` (native 2×) or `general` (native 4× followed by downsampling).

## Results

- `manifest.json`: snapshot of the validated configuration.
- `summary.json`: running/final state, per-job results and per-case timing statistics.
- `summary.csv`: completed/planned counts and median/min/max AI seconds.
- `<case-id>-<repeat>/`: full benchmark output and logs for each job.

Ordinary case failures are recorded and later cases continue. Failed jobs have no timing value and are excluded from statistics. A suite with any failures exits with a nonzero status and `completed_with_errors`; cancellation stops subsequent jobs and records `cancelled`.

Every repeat starts a new model process; timings include model loading and PNG I/O. Driver/disk caches and machine load are not controlled. Compare the same case/settings across repetitions; do not treat different clip lengths or models as a fair speed ranking. No suite result automatically passes the visual-quality gate.
