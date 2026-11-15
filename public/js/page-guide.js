// =============================================================================
// PAGE GUIDE OVERLAY & DIALOG WIDGET
// =============================================================================
//
// Contextual help widget reading route-to-guide rules from `/guides/meta.json`
// and lazy-loading `/guides/*.html` fragments into a native top-layer `<dialog>`.
//
// ## Key Concepts
//
// - **Zero `/pkg` Paths**: Directly resolves `/guides/meta.json` and
//   `/guides/*.html` from `PUBLIC_ASSETS_ROOT` (`public/`).
// - **Zero Polling**: Hooks History API (`pushState`, `replaceState`, `popstate`)
//   to evaluate route patterns in `< 0.01ms`.
// - **Top-Layer Modal (`<dialog closedby="any">`)**: Traps focus via
//   `showModal()`, includes Safari backdrop light-dismiss fallback, and contains
//   background scroll chaining with `overscroll-behavior: contain`.
// - **Header + Floating Trigger**: Renders a collapsible bottom-right `?` Help
//   button on matched routes and exposes `window.openPageGuide()` /
//   `[data-open-page-guide]` triggers.

(function () {
  "use strict";

  const GUIDES_BASE_PATH = "/guides/";
  const META_URL = "/guides/meta.json";
  const STORAGE_COLLAPSED_KEY = "app_page_guide_collapsed";
  const WIDGET_ID = "app-page-guide-widget";
  const DIALOG_ID = "app-page-guide-dialog";
  const STYLE_ID = "app-page-guide-styles";

  /** @type {{ exclude?: string[], guides?: any[] } | null} */
  let metaCache = null;
  /** @type {Promise<any> | null} */
  let metaLoadingPromise = null;
  /** @type {Map<string, string>} */
  const htmlCache = new Map();
  let hasRouteMatch = false;

  function isVietnamese() {
    const lang = (document.documentElement.lang || "en").toLowerCase();
    return lang.startsWith("vi");
  }

  function labels() {
    if (isVietnamese()) {
      return {
        button: "Hướng dẫn",
        buttonTitle: "Mở hướng dẫn trang này",
        collapse: "Thu gọn nút hướng dẫn",
        expand: "Mở nút hướng dẫn",
        defaultTitle: "Hướng dẫn sử dụng trang",
        close: "Đóng",
        loading: "Đang tải hướng dẫn...",
        loadError: "Không thể tải nội dung hướng dẫn. Vui lòng thử lại sau.",
      };
    }
    return {
      button: "Help",
      buttonTitle: "Open page guide",
      collapse: "Collapse help button",
      expand: "Expand help button",
      defaultTitle: "Page Guide",
      close: "Close",
      loading: "Loading guide...",
      loadError: "Unable to load guide content right now. Please try again.",
    };
  }

  function escapeHtml(text) {
    return String(text)
      .replaceAll("&", "&amp;")
      .replaceAll("<", "&lt;")
      .replaceAll(">", "&gt;")
      .replaceAll('"', "&quot;")
      .replaceAll("'", "&#39;");
  }

  function normalizePath(pathname) {
    if (!pathname || pathname === "/") return "/";
    return pathname.replace(/\/+$/, "");
  }

  function matchWildcard(rawPattern, rawPathname, rawSearch) {
    if (!rawPattern) return false;
    const trimmed = rawPattern.trim();
    const pathname = normalizePath(rawPathname);

    const qIdx = trimmed.indexOf("?");
    if (qIdx !== -1) {
      const patPath = normalizePath(trimmed.slice(0, qIdx));
      const patQuery = trimmed.slice(qIdx + 1);
      const fullPattern = patPath + "?" + patQuery;
      const fullTarget = pathname + (rawSearch || "");
      const escapedFull = fullPattern
        .replace(/[.+^${}()|[\]\\?]/g, "\\$&")
        .replace(/\*/g, ".*");
      return new RegExp("^" + escapedFull + "$").test(fullTarget);
    }

    const pattern = normalizePath(trimmed);
    if (pattern === "/*" || pattern === "*") return true;
    if (pattern.endsWith("/*")) {
      const basePattern = pattern.slice(0, -2) || "/";
      if (pathname === basePattern) return true;
    }

    const escaped = pattern
      .replace(/[.+^${}()|[\]\\]/g, "\\$&")
      .replace(/\*/g, ".*")
      .replace(/\?/g, ".");

    return new RegExp("^" + escaped + "$").test(pathname);
  }

  function entryMatchesUrl(entry, pathname, search) {
    if (!entry || (!entry.file && (!Array.isArray(entry.tabs) || !entry.tabs.length))) {
      return false;
    }

    if (Array.isArray(entry.exclude)) {
      if (entry.exclude.some((ex) => matchWildcard(ex, pathname, search))) {
        return false;
      }
    }

    const patterns = Array.isArray(entry.patterns)
      ? entry.patterns
      : typeof entry.path === "string"
        ? [entry.path]
        : [];

    return patterns.some((p) => matchWildcard(p, pathname, search));
  }

  function hasAnyGuideForPath(pathname, search) {
    if (!metaCache || !Array.isArray(metaCache.guides)) return false;
    if (
      Array.isArray(metaCache.exclude) &&
      metaCache.exclude.some((ex) => matchWildcard(ex, pathname, search))
    ) {
      return false;
    }
    return metaCache.guides.some((entry) => entryMatchesUrl(entry, pathname, search));
  }

  function resolveActiveGuideOnOpen(pathname, search) {
    if (!metaCache || !Array.isArray(metaCache.guides)) return null;
    if (
      Array.isArray(metaCache.exclude) &&
      metaCache.exclude.some((ex) => matchWildcard(ex, pathname, search))
    ) {
      return null;
    }

    for (const entry of metaCache.guides) {
      if (!entryMatchesUrl(entry, pathname, search)) continue;
      if (typeof entry.selector === "string" && entry.selector.trim()) {
        try {
          if (!document.querySelector(entry.selector)) continue;
        } catch (err) {
          console.warn("[page-guide] Invalid selector in meta.json:", entry.selector, err);
          continue;
        }
      }
      return entry;
    }
    return metaCache.guides[0] || null;
  }

  async function loadMeta() {
    if (metaCache) return metaCache;
    if (metaLoadingPromise) return metaLoadingPromise;

    metaLoadingPromise = fetch(META_URL, { cache: "no-cache" })
      .then((res) => {
        if (!res.ok) throw new Error("Failed to load " + META_URL);
        return res.json();
      })
      .then((data) => {
        metaCache = data;
        return data;
      })
      .catch((err) => {
        console.warn("[page-guide] Could not load meta.json:", err);
        metaCache = { guides: [] };
        return metaCache;
      })
      .finally(() => {
        metaLoadingPromise = null;
      });

    return metaLoadingPromise;
  }

  async function loadGuideHtml(file) {
    const url =
      file.startsWith("/") || file.startsWith("http")
        ? file
        : GUIDES_BASE_PATH + file.replace(/^\/+/, "");

    if (htmlCache.has(url)) {
      return htmlCache.get(url);
    }

    const res = await fetch(url, { cache: "no-cache" });
    if (!res.ok) {
      throw new Error("HTTP " + res.status);
    }
    const html = await res.text();
    htmlCache.set(url, html);
    return html;
  }

  function ensureStyles() {
    if (document.getElementById(STYLE_ID)) return;

    const style = document.createElement("style");
    style.id = STYLE_ID;
    style.textContent = `
      dialog#${DIALOG_ID}:modal {
        overscroll-behavior: contain;
        margin: auto;
        padding: 0;
        border: none;
        background: transparent;
        max-width: min(52rem, calc(100vw - 2rem));
        max-height: calc(100vh - 2rem);
        width: 100%;
      }
      dialog#${DIALOG_ID}::backdrop {
        overflow: hidden;
        overscroll-behavior: contain;
        background-color: rgba(15, 23, 42, 0.55);
        backdrop-filter: blur(3px);
      }
    `;
    document.head.appendChild(style);
  }

  function ensureDialog() {
    ensureStyles();
    const l = labels();

    let dialog = /** @type {HTMLDialogElement | null} */ (
      document.getElementById(DIALOG_ID)
    );
    if (dialog) return dialog;

    dialog = document.createElement("dialog");
    dialog.id = DIALOG_ID;
    dialog.setAttribute("closedby", "any");
    dialog.setAttribute("aria-labelledby", "app-page-guide-title");
    dialog.className = "rounded-2xl shadow-soft outline-none";

    dialog.innerHTML = `
      <div class="bg-base-sub text-base-content rounded-2xl border border-base-line shadow-soft flex flex-col max-h-[calc(100vh-2rem)] overflow-hidden">
        <div class="px-6 py-4 border-b border-base-line flex items-center justify-between gap-3 bg-base-sub">
          <h3 id="app-page-guide-title" class="text-lg font-bold text-base-content truncate">
            ${escapeHtml(l.defaultTitle)}
          </h3>
          <button
            type="button"
            data-guide-close
            aria-label="${escapeHtml(l.close)}"
            class="w-8 h-8 rounded-xl flex items-center justify-center text-base-content-secondary hover:text-base-content hover:bg-base-main transition-colors cursor-pointer shrink-0"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>
        <div data-guide-tabs class="hidden px-6 pt-3 pb-2.5 border-b border-base-line bg-base-main/40 flex-wrap gap-2"></div>
        <div data-guide-body class="p-6 overflow-y-auto flex-1 min-h-0"></div>
        <div class="px-6 py-3.5 border-t border-base-line bg-base-main/50 flex justify-end">
          <button
            type="button"
            data-guide-close
            class="px-5 py-2 text-sm font-semibold rounded-xl bg-primary text-primary-content hover:bg-primary-hover transition-colors cursor-pointer"
          >
            ${escapeHtml(l.close)}
          </button>
        </div>
      </div>
    `;

    dialog.querySelectorAll("[data-guide-close]").forEach((btn) => {
      btn.addEventListener("click", () => dialog.close());
    });

    if (!("closedBy" in HTMLDialogElement.prototype)) {
      dialog.addEventListener("click", (event) => {
        if (event.target !== dialog) return;
        const rect = dialog.getBoundingClientRect();
        const inside =
          rect.top <= event.clientY &&
          event.clientY <= rect.top + rect.height &&
          rect.left <= event.clientX &&
          event.clientX <= rect.left + rect.width;
        if (!inside) dialog.close();
      });
    }

    document.body.appendChild(dialog);
    return dialog;
  }

  async function renderGuideFileIntoBody(bodyEl, file) {
    if (!bodyEl) return;
    const l = labels();

    bodyEl.innerHTML = `
      <div class="py-8 flex flex-col items-center justify-center gap-2 text-sm text-base-content-secondary">
        <span>${escapeHtml(l.loading)}</span>
      </div>
    `;

    try {
      const html = await loadGuideHtml(file);
      bodyEl.innerHTML = html;
      bodyEl.scrollTop = 0;
    } catch (err) {
      console.warn("[page-guide] Failed to load guide HTML:", err);
      bodyEl.innerHTML = `
        <div class="p-4 rounded-2xl bg-error/10 border border-error/20 text-sm text-base-content">
          ${escapeHtml(l.loadError)}
        </div>
      `;
    }
  }

  async function openGuideModal() {
    await loadMeta();
    const guide = resolveActiveGuideOnOpen(
      window.location.pathname,
      window.location.search
    );
    if (!guide) return;

    const l = labels();
    const dialog = ensureDialog();
    const titleEl = dialog.querySelector("#app-page-guide-title");
    const tabsEl = dialog.querySelector("[data-guide-tabs]");
    const bodyEl = dialog.querySelector("[data-guide-body]");

    if (titleEl) {
      titleEl.textContent = guide.title || l.defaultTitle;
    }

    const tabs =
      Array.isArray(guide.tabs) && guide.tabs.length > 0
        ? guide.tabs
        : guide.file
          ? [{ title: guide.title || l.defaultTitle, file: guide.file }]
          : [];

    if (!tabs.length) return;

    let activeTabIndex = 0;
    for (let i = 0; i < tabs.length; i++) {
      const tabSelector = tabs[i].selector;
      if (typeof tabSelector === "string" && tabSelector.trim()) {
        try {
          if (document.querySelector(tabSelector)) {
            activeTabIndex = i;
            break;
          }
        } catch (_) {}
      }
    }

    function updateTabsUI() {
      if (!tabsEl) return;
      if (tabs.length <= 1) {
        tabsEl.className = "hidden";
        tabsEl.innerHTML = "";
        return;
      }

      tabsEl.className =
        "px-6 pt-3 pb-2.5 border-b border-base-line bg-base-main/40 flex flex-wrap gap-2";
      tabsEl.innerHTML = tabs
        .map((tab, idx) => {
          const active = idx === activeTabIndex;
          const btnClass = active
            ? "px-4 py-2 rounded-xl text-sm font-semibold bg-primary text-primary-content shadow-soft cursor-pointer transition-colors"
            : "px-4 py-2 rounded-xl text-sm font-semibold bg-base-sub text-base-content-secondary border border-base-line hover:text-base-content hover:bg-base-main cursor-pointer transition-colors";
          return `<button type="button" data-tab-index="${idx}" class="${btnClass}">${escapeHtml(tab.title)}</button>`;
        })
        .join("");

      tabsEl.querySelectorAll("[data-tab-index]").forEach((btn) => {
        btn.addEventListener("click", () => {
          const nextIdx = Number(btn.getAttribute("data-tab-index") || 0);
          if (nextIdx === activeTabIndex || !tabs[nextIdx]) return;
          activeTabIndex = nextIdx;
          updateTabsUI();
          renderGuideFileIntoBody(bodyEl, tabs[activeTabIndex].file);
        });
      });
    }

    updateTabsUI();
    if (!dialog.open) {
      dialog.showModal();
    }
    await renderGuideFileIntoBody(bodyEl, tabs[activeTabIndex].file);
  }

  function isCollapsed() {
    try {
      return localStorage.getItem(STORAGE_COLLAPSED_KEY) === "1";
    } catch (_) {
      return false;
    }
  }

  function setCollapsed(collapsed) {
    try {
      localStorage.setItem(STORAGE_COLLAPSED_KEY, collapsed ? "1" : "0");
    } catch (_) {}
    renderWidget();
  }

  function renderWidget() {
    let widget = document.getElementById(WIDGET_ID);
    const l = labels();

    if (!hasRouteMatch) {
      if (widget) widget.remove();
      const dialog = /** @type {HTMLDialogElement | null} */ (
        document.getElementById(DIALOG_ID)
      );
      if (dialog && dialog.open) dialog.close();
      return;
    }

    if (!widget) {
      widget = document.createElement("div");
      widget.id = WIDGET_ID;
      document.body.appendChild(widget);
    }

    if (isCollapsed()) {
      widget.className = "fixed bottom-5 right-0 z-[90] transition-all duration-200";
      widget.innerHTML = `
        <button
          type="button"
          data-action="expand"
          title="${escapeHtml(l.expand)}"
          aria-label="${escapeHtml(l.expand)}"
          class="flex items-center justify-center pl-2 pr-1.5 py-2.5 rounded-l-xl bg-primary text-primary-content shadow-soft hover:bg-primary-hover transition-colors cursor-pointer"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="15 18 9 12 15 6"></polyline>
          </svg>
        </button>
      `;
    } else {
      widget.className = "fixed bottom-5 right-4 z-[90] transition-all duration-200";
      widget.innerHTML = `
        <div class="inline-flex items-stretch rounded-2xl bg-primary text-primary-content shadow-soft overflow-hidden border border-primary-content/15">
          <button
            type="button"
            data-action="open-guide"
            title="${escapeHtml(l.buttonTitle)}"
            class="flex items-center gap-2 pl-3.5 pr-3 py-2 text-xs sm:text-sm font-semibold hover:bg-primary-hover transition-colors cursor-pointer"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4 shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="10"></circle>
              <path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"></path>
              <line x1="12" y1="17" x2="12.01" y2="17"></line>
            </svg>
            <span>${escapeHtml(l.button)}</span>
          </button>
          <button
            type="button"
            data-action="collapse"
            title="${escapeHtml(l.collapse)}"
            aria-label="${escapeHtml(l.collapse)}"
            class="flex items-center justify-center px-2 border-l border-primary-content/20 hover:bg-primary-hover transition-colors cursor-pointer"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="9 18 15 12 9 6"></polyline>
            </svg>
          </button>
        </div>
      `;
    }

    const openBtn = widget.querySelector('[data-action="open-guide"]');
    if (openBtn) openBtn.addEventListener("click", openGuideModal);

    const collapseBtn = widget.querySelector('[data-action="collapse"]');
    if (collapseBtn) {
      collapseBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        setCollapsed(true);
      });
    }

    const expandBtn = widget.querySelector('[data-action="expand"]');
    if (expandBtn) {
      expandBtn.addEventListener("click", (e) => {
        e.stopPropagation();
        setCollapsed(false);
      });
    }
  }

  async function checkRoute() {
    await loadMeta();
    hasRouteMatch = hasAnyGuideForPath(
      window.location.pathname,
      window.location.search
    );
    renderWidget();
  }

  // Expose programmatic API and `[data-open-page-guide]` delegation for header Help buttons.
  window.openPageGuide = openGuideModal;
  document.addEventListener("click", (event) => {
    const target = event.target instanceof Element ? event.target.closest("[data-open-page-guide]") : null;
    if (target) {
      event.preventDefault();
      openGuideModal();
    }
  });

  const originalPushState = history.pushState;
  history.pushState = function () {
    originalPushState.apply(this, arguments);
    checkRoute();
  };

  const originalReplaceState = history.replaceState;
  history.replaceState = function () {
    originalReplaceState.apply(this, arguments);
    checkRoute();
  };

  window.addEventListener("popstate", checkRoute);

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", checkRoute);
  } else {
    checkRoute();
  }
})();
