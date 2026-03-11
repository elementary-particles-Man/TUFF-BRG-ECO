// TUFF selector profile for ChatGPT/Gemini content extraction.
(function () {
  const CHATGPT_HOSTS = ["chatgpt.com", "chat.openai.com"];

  const PROFILE = {
    chatgpt: {
      assistant: [
        '[data-message-author-role="assistant"]',
        'article[data-testid*="conversation-turn"] [data-message-author-role="assistant"]',
        'main article [data-testid*="assistant"]',
        'main article [data-message-author-role="assistant"]'
      ].join(", "),
      user: [
        '[data-message-author-role="user"]',
        'article[data-testid*="conversation-turn"] [data-message-author-role="user"]'
      ].join(", "),
      turns: [
        '[data-message-author-role]',
        'main article',
        'article[data-testid*="conversation-turn"]'
      ].join(", ")
    },
    gemini: {
      assistant: [
        'div[data-message-author-role="model"]',
        'div[data-message-author-role="assistant"]',
        '[data-test-id="model-response"]',
        'model-response',
        'message-content[data-message-author-role="model"]',
        'message-content[data-message-author-role="assistant"]'
      ].join(", "),
      user: [
        'div[data-message-author-role="user"]',
        '[data-test-id="user-query"]',
        'user-query',
        'div[class*="user-query"]'
      ].join(", "),
      turns: [
        'div[data-message-author-role]',
        '[data-test-id="user-query"]',
        '[data-test-id="model-response"]',
        'user-query',
        'model-response',
        'message-content[data-message-author-role]'
      ].join(", ")
    }
  };

  function detectHost(hostname) {
    const host = String(hostname || location.hostname || "").toLowerCase();
    if (CHATGPT_HOSTS.some((h) => host === h || host.endsWith(`.${h}`))) return "chatgpt";
    if (host.includes("gemini.google.com")) return "gemini";
    return "default";
  }

  function selectorSet(hostname) {
    const key = detectHost(hostname);
    return PROFILE[key] || PROFILE.chatgpt;
  }

  function textOf(node) {
    return String((node && (node.innerText || node.textContent)) || "").replace(/\s+/g, " ").trim();
  }

  function hasStreamingMarker(node) {
    if (!(node instanceof HTMLElement)) return false;
    return Boolean(
      node.querySelector(
        '[data-testid="cursor"], [data-testid*="stream"], [data-testid*="typing"], .result-streaming, .typing, .cursor, [class*="streaming"]'
      )
    );
  }

  function pickLatestCompleteAssistantBlock(root) {
    const base = root || document;
    const selectors = selectorSet();
    const nodes = Array.from(base.querySelectorAll(selectors.assistant));
    for (let i = nodes.length - 1; i >= 0; i -= 1) {
      const node = nodes[i];
      if (!(node instanceof HTMLElement)) continue;
      const txt = textOf(node);
      if (!txt || txt.length < 4) continue;
      if (hasStreamingMarker(node)) continue;
      return node;
    }
    return null;
  }

  function latestCompleteAssistantText(root) {
    const node = pickLatestCompleteAssistantBlock(root);
    return node ? textOf(node) : "";
  }

  window.__TUFF_SELECTOR_PROFILE__ = {
    detectHost,
    selectorSet,
    pickLatestCompleteAssistantBlock,
    latestCompleteAssistantText
  };
})();
