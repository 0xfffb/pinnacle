(async function () {
  const VERSION = "1.0.0";
  const COOKIE_CID = "__pinnacle_cid";

  function detectAutomation() {
    const ua = navigator.userAgent || "";
    return {
      cdp: !!(
        window.cdc_adoQpoasnfa76pfcZLmcfl_Array ||
        document.documentElement.getAttribute("cdc") ||
        document.$cdc_asdjflasutopfhvcZLmcfl_
      ),
      webdriver: !!navigator.webdriver,
      headless: /HeadlessChrome/i.test(ua),
      selenium: !!(
        window.__selenium_unwrapped ||
        window._Selenium_IDE_Recorder ||
        document.documentElement.getAttribute("webdriver")
      ),
      phantom: !!(window.callPhantom || window._phantom),
      puppeteer: !!(window.__puppeteer_evaluation_script__ || window.__PUPPETEER_MODULE),
      playwright: !!(
        window.__playwright__binding__ ||
        window.__pwInitScripts ||
        (/Playwright/i.test(ua) && navigator.webdriver)
      ),
    };
  }

  const payload = {
    version: VERSION,
    automation: detectAutomation(),
    seed: Math.random().toString(36).slice(2),
  };

  const cid = document.cookie
    .split(";")
    .map((s) => s.trim())
    .find((s) => s.startsWith(COOKIE_CID + "="));
  if (!cid) {
    location.reload();
    return;
  }

  try {
    await fetch("/__pinnacle", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "same-origin",
      body: JSON.stringify(payload),
    });
  } catch (_) {
    // ignore network errors; reload still happens
  }

  location.reload();
})();
