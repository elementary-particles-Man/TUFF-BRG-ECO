(function () {
  const DEFAULT_BASE = "http://127.0.0.1:8787";

  function normalizeBase(raw) {
    if (typeof raw !== "string") return DEFAULT_BASE;
    const t = raw.trim();
    if (!t) return DEFAULT_BASE;
    return t.replace(/\/+$/, "");
  }

  function openHistory() {
    if (typeof chrome === "undefined" || !chrome.storage || !chrome.tabs) {
      window.open(`${DEFAULT_BASE}/history`, "_blank");
      return;
    }
    chrome.storage.local.get(["TUFF_WEB_BASE"], (res) => {
      const base = normalizeBase(res && res.TUFF_WEB_BASE);
      chrome.tabs.create({ url: `${base}/history` });
      window.close();
    });
  }

  document.getElementById("open-history")?.addEventListener("click", openHistory);
})();
