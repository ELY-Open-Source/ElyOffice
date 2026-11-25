# Sourced: waits for the LOK workers an app's log started.

# True while process $1 runs; Windows pids are native.
lok_worker_alive() {
  if command -v cygpath >/dev/null; then
    tasklist //FI "PID eq $1" //NH 2>/dev/null | grep -qw "$1"
  else
    kill -0 "$1" 2>/dev/null
  fi
}

# Waits up to 30 s for log $1's workers.
lok_workers_gone() {
  local worker
  for worker in $(sed -n 's/.*lok: worker process \([0-9][0-9]*\) started.*/\1/p' "$1"); do
    for _ in $(seq 300); do
      lok_worker_alive "$worker" || continue 2
      sleep 0.1
    done
    echo "FAIL: LOK worker $worker still runs 30 s after its app" >&2
    return 1
  done
}
