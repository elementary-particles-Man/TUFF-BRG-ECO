(function () {
  const DEFAULT_BASE = "http://127.0.0.1:8787";
  const WS_URL = "ws://127.0.0.1:8787";

  function normalizeBase(raw) {
    if (typeof raw !== "string") return DEFAULT_BASE;
    const t = raw.trim();
    if (!t) return DEFAULT_BASE;
    return t.replace(/\/+$/, "");
  }

  function getBase() {
    return new Promise((resolve) => {
      if (typeof chrome === "undefined" || !chrome.storage || !chrome.storage.local) {
        resolve(DEFAULT_BASE);
        return;
      }
      chrome.storage.local.get(["TUFF_WEB_BASE"], (res) => {
        resolve(normalizeBase(res && res.TUFF_WEB_BASE));
      });
    });
  }

  async function openHistory() {
    const base = await getBase();
    if (typeof chrome !== "undefined" && chrome.tabs) {
      chrome.tabs.create({ url: `${base}/history` });
      window.close();
    } else {
      window.open(`${base}/history`, "_blank");
    }
  }

  async function fetchPending() {
    const base = await getBase();
    const res = await fetch(`${base}/facts/pending`, { cache: "no-store" });
    if (!res.ok) return [];
    return await res.json();
  }

  function sendWsMessage(message) {
    return new Promise((resolve, reject) => {
      let ws;
      try {
        ws = new WebSocket(WS_URL);
      } catch (e) {
        reject(e);
        return;
      }
      const timer = setTimeout(() => {
        try { ws.close(); } catch (_) {}
        reject(new Error("ws timeout"));
      }, 2500);

      ws.onopen = () => {
        ws.send(JSON.stringify(message));
        setTimeout(() => {
          clearTimeout(timer);
          try { ws.close(); } catch (_) {}
          resolve();
        }, 180);
      };
      ws.onerror = (e) => {
        clearTimeout(timer);
        reject(e);
      };
    });
  }

  async function approve(id) {
    await sendWsMessage({
      type: "ApproveFact",
      id: crypto.randomUUID(),
      ts: new Date().toISOString(),
      payload: { id }
    });
  }

  async function repropose(tag, value) {
    await sendWsMessage({
      type: "ProposeFact",
      id: crypto.randomUUID(),
      ts: new Date().toISOString(),
      payload: { tag, value }
    });
  }

  function renderList(items) {
    const list = document.getElementById("list");
    list.innerHTML = "";
    if (!Array.isArray(items) || items.length === 0) {
      const empty = document.createElement("div");
      empty.className = "muted";
      empty.textContent = "pending はありません";
      list.appendChild(empty);
      return;
    }

    items.forEach((item) => {
      const card = document.createElement("div");
      card.className = "item";

      const tag = document.createElement("div");
      tag.className = "tag";
      tag.textContent = item.tag || "(no-tag)";

      const val = document.createElement("div");
      val.className = "val";
      val.textContent = item.value || "";

      const actions = document.createElement("div");
      actions.className = "actions";

      const edit = document.createElement("button");
      edit.className = "btn";
      edit.textContent = "Edit";
      edit.addEventListener("click", async () => {
        const newTag = prompt("Tag", item.tag || "");
        if (!newTag) return;
        const newVal = prompt("Value", item.value || "");
        if (!newVal) return;
        await repropose(newTag, newVal);
        await load();
      });

      const approveBtn = document.createElement("button");
      approveBtn.className = "btn ok";
      approveBtn.textContent = "Approve";
      approveBtn.addEventListener("click", async () => {
        await approve(item.id);
        await load();
      });

      actions.append(edit, approveBtn);
      card.append(tag, val, actions);
      list.appendChild(card);
    });
  }

  async function load() {
    try {
      const items = await fetchPending();
      renderList(items);
    } catch (e) {
      console.warn("failed to load pending", e);
      renderList([]);
    }
  }

  document.getElementById("open-history")?.addEventListener("click", openHistory);
  document.getElementById("refresh")?.addEventListener("click", load);
  load();
})();
