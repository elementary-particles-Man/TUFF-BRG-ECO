const WS_URL = "ws://127.0.0.1:8787";
const MENU_ID = "tuff-register-fact";

function normalizeTag(input) {
  const s = String(input || "")
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9\u3040-\u30ff\u3400-\u9fff]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
  return s.slice(0, 64) || "fact";
}

function parseSelection(text) {
  const raw = String(text || "").trim();
  if (!raw) return null;
  if (raw.includes("\t")) {
    const [tag, ...rest] = raw.split("\t");
    const value = rest.join("\t").trim();
    if (value) return { tag: normalizeTag(tag), value };
  }
  if (raw.includes("=")) {
    const idx = raw.indexOf("=");
    const tag = raw.slice(0, idx).trim();
    const value = raw.slice(idx + 1).trim();
    if (value) return { tag: normalizeTag(tag), value };
  }
  const value = raw.replace(/\s+/g, " ").trim();
  const head = value.slice(0, 18);
  return { tag: normalizeTag(head), value };
}

function sendProposeFact(payload) {
  return new Promise((resolve, reject) => {
    let settled = false;
    let ws;
    try {
      ws = new WebSocket(WS_URL);
    } catch (e) {
      reject(e);
      return;
    }

    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      try { ws.close(); } catch (_) {}
      reject(new Error("WebSocket timeout"));
    }, 2500);

    ws.onopen = () => {
      ws.send(JSON.stringify({
        type: "ProposeFact",
        id: crypto.randomUUID(),
        ts: new Date().toISOString(),
        payload
      }));
      setTimeout(() => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        try { ws.close(); } catch (_) {}
        resolve();
      }, 180);
    };

    ws.onerror = (err) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(err);
    };
  });
}

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: MENU_ID,
    title: "TUFF: Register as Fact",
    contexts: ["selection"]
  });
});

chrome.contextMenus.onClicked.addListener(async (info) => {
  if (info.menuItemId !== MENU_ID) return;
  const parsed = parseSelection(info.selectionText || "");
  if (!parsed) return;
  try {
    await sendProposeFact(parsed);
  } catch (e) {
    console.warn("TUFF propose failed", e);
  }
});
