// Attach delegated handlers for CSP-safe actions
(function () {
  function copyText(text) {
    if (!navigator.clipboard)
      return Promise.reject(new Error("Clipboard API not available"));
    return navigator.clipboard.writeText(text);
  }

  document.addEventListener(
    "click",
    function (e) {
      var btn = e.target.closest('[data-action="copy-short-url"]');
      if (btn) {
        e.preventDefault();
        var payload = btn.getAttribute("data-copy");
        if (!payload) return;
        copyText(payload)
          .then(function () {
            var original = btn.textContent;
            btn.textContent = "COPIED!";
            setTimeout(function () {
              btn.textContent = original;
            }, 1500);
          })
          .catch(function () {
            console.error("Failed to copy");
          });
      }
    },
    false,
  );
})();
