// Render ```mermaid code blocks with mermaid.min.js (loaded as the
// previous additional-js entry, so `mermaid` is already on window).
//
// mdbook turns fenced ```mermaid into <pre><code class="language-mermaid">…
// which mermaid won't pick up automatically. Convert each occurrence to
// the <div class="mermaid"> shape mermaid.run() expects, then render.
//
// Theme follows mdbook's html.classList — light/coal/navy/ayu/rust ↔
// mermaid's "default" or "dark".

(function () {
  if (typeof window === "undefined" || typeof document === "undefined") return;
  if (typeof mermaid === "undefined") return;

  function mermaidThemeFor(htmlClass) {
    var dark = ["coal", "navy", "ayu"];
    for (var i = 0; i < dark.length; i++) {
      if (htmlClass.indexOf(dark[i]) !== -1) return "dark";
    }
    return "default";
  }

  function convertCodeBlocks() {
    var nodes = document.querySelectorAll("pre code.language-mermaid");
    for (var i = 0; i < nodes.length; i++) {
      var code = nodes[i];
      var pre = code.parentNode;
      var div = document.createElement("div");
      div.className = "mermaid";
      div.textContent = code.textContent;
      pre.parentNode.replaceChild(div, pre);
    }
  }

  function init() {
    var theme = mermaidThemeFor(document.documentElement.className || "");
    mermaid.initialize({
      startOnLoad: false,
      theme: theme,
      securityLevel: "loose", // allow click handlers and links inside diagrams
      flowchart: { htmlLabels: true, curve: "basis" },
    });
    convertCodeBlocks();
    mermaid.run({ querySelector: ".mermaid" }).catch(function (err) {
      // Swallow render errors so a malformed diagram never breaks the page chrome.
      console.warn("mermaid render failed:", err);
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", init);
  } else {
    init();
  }
})();
