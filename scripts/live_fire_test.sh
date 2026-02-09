#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT_DIR"

LOG_FILE="/tmp/tuff_brg_live_fire.log"
MEANING_DIR="_tuffdb/lightweight"
MEANING_FILE="${MEANING_DIR}/meaning.db"
PID=""

cleanup() {
  if [[ -n "${PID}" ]] && kill -0 "${PID}" 2>/dev/null; then
    kill -TERM "${PID}" 2>/dev/null || true
    wait "${PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

mkdir -p "${MEANING_DIR}"
cat > "${MEANING_FILE}" <<'DB'
prime-minister=Sanae Takaichi
shigeru-ishiba=Former Candidate
japan-leader=Takaichi
DB

: >"${LOG_FILE}"
RUST_LOG=debug cargo build -p tuff_brg >>"${LOG_FILE}" 2>&1
TUFF_FAST_PATH=1 \
TUFF_LIGHTWEIGHT_MEANING_PATH="${MEANING_FILE}" \
RUST_LOG=debug ./target/debug/tuff_brg >>"${LOG_FILE}" 2>&1 &
PID=$!

for _ in $(seq 1 240); do
  if (echo > /dev/tcp/127.0.0.1/8787) >/dev/null 2>&1; then
    break
  fi
  sleep 0.25
done

if ! (echo > /dev/tcp/127.0.0.1/8787) >/dev/null 2>&1; then
  echo "ERROR: tuff_brg did not start on 127.0.0.1:8787"
  exit 1
fi

node <<'NODE'
function openSocket(url) {
  if (typeof WebSocket !== "undefined") return new WebSocket(url);
  const Ws = require("ws");
  return new Ws(url);
}

function wsSendAndCollect(fragment) {
  return new Promise((resolve, reject) => {
    const ws = openSocket("ws://127.0.0.1:8787/");
    const out = [];
    const timeout = setTimeout(() => {
      try { ws.close(); } catch (_) {}
      resolve(out);
    }, 1600);

    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: "StreamFragment",
        id: crypto.randomUUID(),
        ts: new Date().toISOString(),
        payload: {
          conversation_id: "live-fire",
          sequence_number: 1,
          url: "https://example.test",
          selector: "#model",
          fragment,
          context: { page_title: "LiveFire", locale: "en-US" }
        }
      }));
    };

    ws.onmessage = (ev) => {
      try { out.push(JSON.parse(ev.data)); } catch (_) {}
    };

    ws.onerror = (err) => {
      clearTimeout(timeout);
      reject(err);
    };

    ws.onclose = () => {
      clearTimeout(timeout);
      resolve(out);
    };
  });
}

function hasStop(messages) {
  return messages.some((m) => m.type === "ControlCommand" && m.payload && m.payload.command === "STOP");
}

function findJudge(messages) {
  return messages.find((m) => m.type === "JudgeResult" && m.payload);
}

(async () => {
  const bad = await wsSendAndCollect("prime-minister\tThe current Prime Minister of Japan is Shigeru Ishiba.");
  const badJudge = findJudge(bad);
  if (!badJudge) {
    throw new Error("No JudgeResult for hallucination case");
  }
  if (!["SMOKE", "GRAY_BLACK"].includes(badJudge.payload.status)) {
    throw new Error(`Expected SMOKE/GRAY_BLACK for hallucination, got ${badJudge.payload.status}`);
  }
  if (!hasStop(bad)) {
    throw new Error("Expected STOP control for hallucination case");
  }

  const good = await wsSendAndCollect("prime-minister\tPrime Minister Sanae Takaichi announced policy updates.");
  const goodJudge = findJudge(good);
  if (!goodJudge) {
    throw new Error("No JudgeResult for valid case");
  }
  if (!["WHITE", "GRAY_WHITE"].includes(goodJudge.payload.status)) {
    throw new Error(`Expected WHITE/GRAY_WHITE for valid case, got ${goodJudge.payload.status}`);
  }

  console.log("OK: live fire test passed");
})().catch((e) => {
  console.error(String(e && e.message ? e.message : e));
  process.exit(1);
});
NODE

kill -INT "${PID}"
set +e
wait "${PID}"
exit_code=$?
set -e
PID=""

if [[ "${exit_code}" -ne 0 ]]; then
  echo "ERROR: process exited with code ${exit_code}"
  exit 1
fi

echo "OK: live fire script completed"
